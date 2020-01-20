use std::fs::File;
use std::io::BufWriter;
use std::io::Error;
use std::io::ErrorKind;

use memmap::Mmap;
use serde::de::DeserializeOwned;
use serde::Serialize;

fn convert<T>(from: &str, to: &str) -> Result<(), Error>
where
    T: Serialize + DeserializeOwned,
{
    let file_in = File::open(from)?;
    let file_out = File::create(to)?;

    // We create a buffered writer from the file we get
    let writer = BufWriter::new(&file_out);

    let mmap = unsafe { Mmap::map(&file_in)? };
    let v: T = serde_json::from_slice(&mmap[..])?;

    match bincode::serialize_into(writer, &v) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Bincode could not serialize: {:?}", e),
        )),
    }
}

pub fn from<T>(name: &str) -> Result<(), Error>
where
    T: Serialize + DeserializeOwned,
{
    // Convert definitions from json to bincode
    let fn_in = format!("{}.json", name);
    let fn_out = format!("{}.bin", name);

    convert::<T>(&fn_in, &fn_out)
}
