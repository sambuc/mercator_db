//! # XYZ file format
//!
//! This module support reading files read by [MeshView] tool used at
//! the [University of Oslo].
//!
//! # File structure
//!
//! Each files begins with:
//!
//! ```txt
//! RGBA [Red] [Green] [Blue] [Alpha] # RGBA
//! [X],[Y],[Z] # WHS Origin
//! [X],[Y],[Z] # Bregma
//!
//! SCALE [F]
//! ```
//!
//!  * `RGBA [Red] [Green] [Blue] [Alpha]`: defines the color to use for
//!    the following points
//!  * `[X],[Y],[Z] # WHS Origin`: defines where the Waxholm Origin is
//!    in Voxel coordinates.
//!  * `[X],[Y],[Z] # Bregma`: same as above, for another reference
//!    space.
//!  * `SCALE [F]`: **TBC** Size of the voxels.
//!
//! The rest of the file contains (one per line):
//!  * coordinate triplets (x, y and z), each  representing one point
//!    coordinate.
//!  * `RGB [Red] [Green] [Blue]`: Which applies from that line
//!    until further notice.
//!  * A comment Line, starting with `#`
//!
//! ## File Coordinate system
//!
//! Coordinates in MeshView follow RAS (Right-Anterior-Superior)
//! orientation and are expressed in voxels:
//!  * First axis `x` starts from the left side of the volume, and
//!    points towards the right.
//!  * Second axis `y` starts from the backmost position in the volume,
//!    and points towards the front.
//!  * Third axis `z` starts from the bottom of the volume and points
//!    towards the top.
//!
//! # Waxholm Space
//!
//! ## Conversion to Waxholm Space
//!
//! The [Waxholm Space Atlas] of the Sprague Dawley Rat Brain (WHS) uses
//! the same axis order and orientation as the MeshView tool, there is
//! only a translation of the origin, and scaling have to be applied.
//!
//! # Example
//!
//! ```txt
//! RGBA 1 0 0 1 # RGBA
//! 244,623,248 # WHS Origin
//! 246,653,440 # Bregma
//!
//! #Aar27s49 26 0
//! RGB 0.12941176470588237 0.403921568627451 0.1607843137254902
//! 221.40199877 413.34541500312037 172.79973508489095
//! 220.5800097805 412.82939421970866 173.56428074436994
//!
//! #Aar27s48 49 0
//! RGB 0.12941176470588237 0.403921568627451 0.1607843137254902
//! 237.35325687425 412.5720395183866 176.6713556605702
//! ```
//!
//! ## Conversion to Waxholm
//!
//! Assuming the following extents of "WHS Rat 39 μm" in voxels:
//!
//!  * Leftmost sagittal plane: `x = 0`
//!  * Backmost coronal plane: `y = 0`
//!  * Bottommost horizontal plane: `z = 0`
//!  * Rightmost sagittal plane: `x = 511`
//!  * Frontmost coronal plane: `y = 1023`
//!  * Topmost horizontal plane: `z = 511`
//!
//! **NOTE**: Directions are deliberately matching the default
//!           orientation of ​NIfTI​ data.
//!
//! 1. As per the `WHS Origin` directive, it is at 244, 623, 248 voxel
//!    coordinates, which means each coordinate must be subtracted with
//!    the corresponding value, then
//! 2. the coordinates must be converted to millimeters, a.k.a
//!    multiplied by the atlas resolution. For the atlas of this example
//!    it is 0.0390625 [mm], isotropic.
//!
//! This gives us the following conversion formula:
//!
//! ```txt
//!                                   ⎡ 0.0390625  0          0         0 ⎤
//! [ xw yw zw 1 ] = [ xq yq zq 1 ] * ⎢ 0          0.0390625  0         0 ⎥
//!                                   ⎢ 0          0          0.0390625 0 ⎥
//!                                   ⎣ -9.53125 -24.3359375 -9.6875    1 ⎦
//! ```
//!
//! Where:
//!  * `[x​w​, y​w​, z​w 1]​` are WHS coordinates (RAS directions, expressed
//!    in millimeters).
//!  * `[x​q​, y​q, z​q 1]`​ are MeshView coordinates for the **WHS Rat 39 μm**
//!    package (RAS directions, expressed in 39.0625 μm voxels).
//!
//!
//!
//! [MeshView]: http://www.nesys.uio.no/MeshView/meshview.html?atlas=WHS_SD_rat_atlas_v2
//! [University of Oslo]: https://www.med.uio.no/imb/english/research/groups/neural-systems/index.html
//! [Waxholm Space Atlas]: https://www.nitrc.org/projects/whs-sd-atlas

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

/// Read a XYZ file and convert it to the internal format for indexing.
///
/// This only converts the data point definitions, a reference space
/// needs to be provided as well to be able to build an index.
///
///  # Parameters
///
///  * `name`:
///      Base name of the file,
///       * `.xyz` will be automatically appended for the source file, while
///       * `.bin` will be appended for the output file.
pub fn from(name: &str) -> Result<(), Error> {
    let fn_in = format!("{}.xyz", name);
    let fn_out = format!("{}.bin", name);

    let mut file_in = BufReader::new(File::open(&fn_in)?);
    let mut string = String::new();
    file_in.read_to_string(&mut string)?;

    let v = convert(&string)?;

    store(v, &fn_out)
}
