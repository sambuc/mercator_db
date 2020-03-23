//! Bincode support

use std::fs::File;
use std::io::BufWriter;
use std::io::Error;
use std::io::ErrorKind;

use memmap::Mmap;
use serde::de::DeserializeOwned;
use serde::Serialize;

use super::model;

/// Deserialize a data structure.
///
/// # Parameters
///
///  * `from`:
///      File to read, which contains Bincode data.
pub fn load<T>(from: &str) -> Result<T, Error>
where
    T: DeserializeOwned,
{
    let file_in = File::open(from)?;

    let mmap = unsafe { Mmap::map(&file_in)? };

    match bincode::deserialize(&mmap[..]) {
        Ok(data) => Ok(data),
        Err(e) => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Bincode could not deserialize: {:?}", e),
        )),
    }
}

/// Serialize a data structure.
///
/// # Parameters
///
///  * `data`:
///      Data to serialize.
///
///  * `to`:
///      File to use to store the serialized data.
pub fn store<T>(data: T, to: &str) -> Result<(), Error>
where
    T: Serialize,
{
    let file_out = File::create(to)?;

    // We create a buffered writer from the file we get
    let writer = BufWriter::new(&file_out);

    match bincode::serialize_into(writer, &data) {
        Ok(()) => Ok(()),
        Err(e) => Err(Error::new(
            ErrorKind::InvalidData,
            format!("Bincode could not serialize: {:?}", e),
        )),
    }
}

/// Build an index from the input files.
///
/// # Parameters
///
///  * `name`:
///      Index name, this value will also be used to generate file names
///      as such:
///       * `.spaces.bin` and `.objects.bin` will be appended for the
///          input files.
///       * `.index` will be appended for the index file.
///
/// * `version`:
///     Parameter to distinguish revisions of an index.
///
/// * `scales`:
///     An optional list of specific index resolutions to generates on
///     top of the full resolution one.
///
/// * `max_elements`:
///     If this is specified, automatically generates scaled indices, by
///     halving the number elements between resolutions, and stop
///     generating indices either when the number of points remaining is
///     equal to the number of distinct Ids, or smaller or equal to this
///     value.
///
/// **Note**: `max_elements` is ignored when `scales` is not `None`.
pub fn build(
    name: &str,
    version: &str,
    scales: Option<Vec<Vec<u32>>>,
    max_elements: Option<usize>,
) -> Result<(), Error> {
    let fn_spaces = format!("{}.spaces.bin", name);
    let fn_objects = format!("{}.objects.bin", name);
    let fn_index = format!("{}.index", name);

    let spaces = load::<Vec<model::Space>>(&fn_spaces)?
        .iter()
        .map(|s| s.into())
        .collect::<Vec<_>>();

    let objects = load::<Vec<model::SpatialObject>>(&fn_objects)?;

    let core = match model::build_index(name, version, &spaces, &objects, scales, max_elements) {
        Ok(core) => core,
        Err(e) => {
            return Err(Error::new(
                ErrorKind::InvalidData,
                format!("Failure to build index: {:?}", e),
            ))
        }
    };

    store((spaces, core), &fn_index)
}
