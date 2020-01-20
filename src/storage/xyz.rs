use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Error;
use std::io::ErrorKind;
use std::io::Read;

use super::bincode::store;
use super::model::v1::Shape;
use super::model::v1::SpatialObject;
use super::model::Properties;

fn convert(string: &str) -> Result<Vec<SpatialObject>, Error> {
    // Read manually the XYZ file, as this is a simple format.
    // Read line by line, skip all line we don't know how to parse, for the
    // remaining ones do:
    //  * lines starting with '#A' we update the current point ID
    //  * lines we can parse as triplet fo f64, add a position to the list,
    //     under the oid key.
    let mut oids = HashMap::new();
    let mut oid = None;
    let mut origin = vec![];
    for line in string.lines() {
        let values = line.split_whitespace().collect::<Vec<_>>();

        if values.is_empty() {
            // Skip empty lines
            continue;
        }

        match values[0] {
            "RGBA" => (),
            "RGB" => (),
            "SCALE" => (),
            _ if values[0].starts_with("#A") => {
                // Update the oid value.
                oid = Some(values[0].trim_start_matches('#').to_string());
                trace!("FOUND OID {:?}", oid);
            }
            _ if line.contains("WHS") => {
                // Store the voxel offset value
                let t: Vec<_> = values[0]
                    .split(',')
                    .filter_map(|s| match s.parse::<f64>() {
                        Err(_) => None,
                        Ok(v) => Some(v),
                    })
                    .collect();

                if t.len() == 3 && origin.is_empty() {
                    origin = t;
                } else {
                    return Err(Error::new(
                        ErrorKind::InvalidData,
                        format!("Invalid WHS origin new {:?}, current {:?}", t, origin),
                    ));
                }
                trace!("ORIGIN FOUND: {:?}", origin);
            }
            _ if values.len() == 3 => {
                // Check we have an oid to register the position under first.

                let x = values[0].parse::<f64>();
                let y = values[1].parse::<f64>();
                let z = values[2].parse::<f64>();

                if let (Some(oid), Ok(x), Ok(y), Ok(z)) = (oid.clone(), x, y, z) {
                    trace!("after (oid, x, y, z) = {:?}", (&oid, &x, &y, &z));
                    // We need to convert these voxel values into mm-s
                    let (x, y, z) = (x - origin[0], y - origin[1], z - origin[2]);
                    let (x, y, z) = (x * 0.039_062_5, y * 0.039_062_5, z * 0.039_062_5);

                    oids.entry(oid)
                        .or_insert_with(|| vec![])
                        .push(vec![x, y, z]);
                }
            }
            _ => trace!("line {:?}, values: {:?}", line, values),
        }
    }

    // Transform the points into SpatialObjects
    Ok(oids
        .drain()
        .map(|(k, v)| {
            let properties = Properties {
                type_name: "Feature".to_string(),
                id: k,
            };

            let shapes = v
                .into_iter()
                .map(|position| Shape {
                    type_name: "Point".to_string(),
                    reference_space: "WHS-Rat-um".to_string(),
                    vertices: vec![position],
                })
                .collect();

            SpatialObject { properties, shapes }
        })
        .collect())
}

pub fn from(name: &str) -> Result<(), Error> {
    // Convert Reference Space definitions
    let fn_in = format!("{}.xyz", name);
    let fn_out = format!("{}.bin", name);

    let mut file_in = BufReader::new(File::open(&fn_in)?);
    let mut string = String::new();
    file_in.read_to_string(&mut string)?;

    let v = convert(&string)?;

    store(v, &fn_out)
}
