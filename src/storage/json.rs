use std::fs::File;
use std::io::BufWriter;

use memmap::Mmap;
use serde::de::DeserializeOwned;
use serde::Serialize;

fn convert<T>(from: &str, to: &str)
where
    T: Serialize + DeserializeOwned,
{
    let file_in =
        File::open(from).unwrap_or_else(|e| panic!("Unable to read file: {}: {}", from, e));
    let file_out =
        File::create(to).unwrap_or_else(|e| panic!("Unable to create file: {}: {}", to, e));

    // We create a buffered writer from the file we get
    let writer = BufWriter::new(&file_out);

    let mmap = unsafe {
        Mmap::map(&file_in)
            .unwrap_or_else(|e| panic!("Unable to map in memory the file: {}: {}", from, e))
    };
    let v: T = serde_json::from_slice(&mmap[..])
        .unwrap_or_else(|e| panic!("Unable to parse the json data from: {}: {}", from, e));

    bincode::serialize_into(writer, &v).unwrap();
}

pub fn from<T>(name: &str)
where
    T: Serialize + DeserializeOwned,
{
    // Convert definitions from json to bincode
    let fn_in = format!("{}.json", name);
    let fn_out = format!("{}.bin", name);

    convert::<T>(&fn_in, &fn_out);
}
