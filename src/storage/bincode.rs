use std::fs::File;
use std::io::BufWriter;

use memmap::Mmap;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::model;

pub fn load<T>(from: &str) -> T
where
    T: DeserializeOwned,
{
    let file_in =
        File::open(from).unwrap_or_else(|e| panic!("Unable to read file: {}: {}", from, e));

    let mmap = unsafe {
        Mmap::map(&file_in)
            .unwrap_or_else(|e| panic!("Unable to map in memory the file: {}: {}", from, e))
    };

    bincode::deserialize(&mmap[..])
        .unwrap_or_else(|e| panic!("Unable to parse the json data from: {}: {}", from, e))
}

pub fn store<T>(data: T, to: &str)
where
    T: Serialize,
{
    let file_out =
        File::create(to).unwrap_or_else(|e| panic!("Unable to create file: {}: {}", to, e));

    // We create a buffered writer from the file we get
    let writer = BufWriter::new(&file_out);

    bincode::serialize_into(writer, &data).unwrap();
}

pub fn build(
    name: &str,
    version: &str,
    scales: Option<Vec<Vec<u32>>>,
    max_elements: Option<usize>,
) {
    let fn_spaces = format!("{}.spaces.bin", name);
    let fn_objects = format!("{}.objects.bin", name);
    let fn_index = format!("{}.index", name);

    let spaces = load::<Vec<model::Space>>(&fn_spaces)
        .iter()
        .map(|s| s.into())
        .collect::<Vec<_>>();

    let objects = load::<Vec<model::SpatialObject>>(&fn_objects);

    let core = model::build_index(name, version, &spaces, &objects, scales, max_elements);

    store((spaces, core), &fn_index);
}
