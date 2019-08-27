use memmap::Mmap;
use serde::Deserialize;
use std::fs::File;
use std::io::BufWriter;

const K: usize = 3;

#[derive(Serialize, Deserialize, Debug)]
pub struct Properties {
    pub id: String,
}

#[derive(Serialize, Deserialize, Debug)]
// Geometry is parametric as we have a specific deserializer for the JSON format.
pub struct Shape<'a, G> {
    #[serde(rename = "type")]
    pub type_name: &'a str,
    pub geometry: G,
    pub properties: Properties,
}

pub mod json {
    use super::*;

    use serde::Deserializer;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Geometry<'a> {
        #[serde(rename = "type")]
        pub type_name: &'a str,
        #[serde(rename = "referenceSpace")]
        pub reference_space: &'a str,
        #[serde(deserialize_with = "deserialize_coordinates")]
        pub coordinates: Vec<[f64; K]>,
    }

    fn deserialize_coordinates<'de, D>(deserializer: D) -> Result<Vec<[f64; K]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        // Retrieve from the deserializer a vector of Strings, it is important to specify both the type
        // of elements in the vector and use `Vec::` to obtain a vector from the json input.
        // Vec<String> corresponds to ["0.1,0.1,0.1", ...] of the input.
        let strings: Vec<String> = Vec::deserialize(deserializer)?;
        let mut shape_coords = vec![];

        // For each string, decompose into a fixed point float. A string might have multiple dimensions,
        // we are generic in this regards, although we do not check for each point to be have a constant
        // number of dimensions.
        for pos_string in &strings {
            // split the string on the `,`, convert each part to float, and store the vector.
            let pos_float: Vec<f64> = pos_string
                .split(',')
                .map(move |a| a.parse::<f64>().unwrap())
                .collect();

            assert_eq!(pos_float.len(), K);

            shape_coords.push(*array_ref![pos_float, 0, K])
        }
        Ok(shape_coords)
    }

    pub fn convert(from: &str, to: &str) {
        let file_in = File::open(from).unwrap();
        let file_out = File::create(to).expect("Unable to create file");

        // We create a buffered writer from the file we get
        let writer = BufWriter::new(&file_out);

        let mmap = unsafe { Mmap::map(&file_in).unwrap() };
        let v: Vec<Shape<Geometry>> = serde_json::from_slice(&mmap[..]).unwrap();

        bincode::serialize_into(writer, &v).unwrap();
    }
}

pub mod bin {
    use super::*;

    use mercator_db::space;
    use mercator_db::Core;
    use mercator_db::DataBase;
    use mercator_db::Properties;
    use mercator_db::SpaceSetObject;

    use std::collections::HashMap;

    #[derive(Serialize, Deserialize, Debug)]
    pub struct Geometry<'a> {
        pub type_name: &'a str,
        pub reference_space: &'a str,
        pub coordinates: Vec<[f64; K]>,
    }

    pub fn build(from: &str, to: &str) {
        let file_in = File::open(from).unwrap();
        let file_out = File::create(to).expect("Unable to create file");

        // We create a buffered writer from the file we get
        let writer = BufWriter::new(&file_out);

        let mmap = unsafe { Mmap::map(&file_in).unwrap() };
        let v: Vec<Shape<Geometry>> = bincode::deserialize(&mmap[..]).unwrap();

        let mut spaces = vec![];
        let mut properties = vec![];
        let mut space_set_objects = Vec::with_capacity(v.len());

        {
            let mut properties_hm = HashMap::new();
            let mut space_ids = HashMap::new();

            let mut properties_ref = Vec::with_capacity(v.len());

            // What to write in binary, a vec of json::shape or a Vec of SpaceShape?
            for shape in &v {
                assert!(shape.type_name == "Feature");
                assert!(shape.geometry.type_name == "Point");

                space_ids.insert(shape.geometry.reference_space, 1u8);

                // Check if a properties Object exists, if not create it, keep an
                // offset to a reference to that Properties.
                // We store a new reference into a reference list, so that, we can
                // later on build a deduplicated list and keep stable references.
                // FIXME: Comment unclear
                let value = match properties_hm.get(shape.properties.id.as_str()) {
                    Some(_) => {
                        properties_ref.push(shape.properties.id.as_str());
                        properties_ref.len() - 1
                    }
                    None => {
                        properties_hm.insert(
                            shape.properties.id.as_str(),
                            Properties::Feature(shape.properties.id.clone()),
                        );

                        properties_ref.push(shape.properties.id.as_str());
                        properties_ref.len() - 1
                    }
                };

                space_set_objects.push(SpaceSetObject::new(
                    shape.geometry.reference_space,
                    shape.geometry.coordinates[0].to_vec().into(),
                    value.into(),
                ));
            }

            properties.append(&mut properties_hm.drain().map(|(_, v)| v).collect::<Vec<_>>());

            spaces.append(
                &mut space_ids
                    .keys()
                    .map(|&space_name| {
                        space::Space::new(
                            space_name,
                            space::CoordinateSystem::new(
                                vec![0f64, 0f64, 0f64],
                                vec![
                                    space::Axis::new(
                                        "m",
                                        vec![1f64, 0f64, 0f64],
                                        space::NumberSet::N,
                                        0.0,
                                        1.0,
                                        1E9 as u64,
                                    )
                                    .unwrap(),
                                    space::Axis::new(
                                        "m",
                                        vec![0f64, 1f64, 0f64],
                                        space::NumberSet::N,
                                        0.0,
                                        1.0,
                                        1E9 as u64,
                                    )
                                    .unwrap(),
                                    space::Axis::new(
                                        "m",
                                        vec![0f64, 0f64, 1f64],
                                        space::NumberSet::N,
                                        0.0,
                                        1.0,
                                        1E9 as u64,
                                    )
                                    .unwrap(),
                                ],
                            ),
                        )
                    })
                    .collect::<Vec<_>>(),
            );

            properties.sort_unstable_by_key(|p| p.id().clone());

            space_set_objects.iter_mut().for_each(|object| {
                let id = properties_ref[object.value().u64() as usize];
                let value = properties.binary_search_by_key(&id, |p| p.id()).unwrap();
                object.set_value(value.into());
            });
        }

        let cores = vec![Core::new(
            "test",
            "v0.1",
            &spaces,
            properties,
            space_set_objects,
        )];

        let db = DataBase::new(spaces, cores);
        bincode::serialize_into(writer, &db).unwrap();
    }
}

pub fn convert<S>(name: S)
where
    S: Into<String>,
{
    let name = name.into();
    let fn_in = format!("{}.json", name);
    let fn_out = format!("{}.bin", name);

    json::convert(&fn_in, &fn_out);
}

pub fn build<S>(name: S)
where
    S: Into<String>,
{
    let name = name.into();
    let fn_in = format!("{}.bin", name);
    let fn_out = format!("{}.index", name);

    bin::build(&fn_in, &fn_out);
}
