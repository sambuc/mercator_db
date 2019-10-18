use std::collections::HashMap;

use crate::database;
use database::space;
use database::Core;
use database::DataBase;
use database::SpaceObject;
use database::SpaceSetObject;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Space {
    pub name: String,
    pub origin: Vec<f64>,
    pub axes: Vec<Axis>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Axis {
    pub measurement_unit: String,
    pub graduation: Graduation,
    pub unit_vector: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Graduation {
    pub set: String,
    pub minimum: f64,
    pub maximum: f64,
    pub steps: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpatialObject {
    pub properties: Properties,
    pub shapes: Vec<Shape>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Shape {
    #[serde(rename = "type")]
    pub type_name: String,
    #[serde(rename = "space")]
    pub reference_space: String,
    pub vertices: Vec<Point>,
}

type Point = Vec<f64>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Properties {
    #[serde(rename = "type")]
    pub type_name: String,
    pub id: String,
}

impl From<&space::Graduation> for Graduation {
    fn from(g: &space::Graduation) -> Self {
        Graduation {
            set: g.set.clone().into(),
            minimum: g.minimum,
            maximum: g.maximum,
            steps: g.steps,
        }
    }
}

impl From<Axis> for space::Axis {
    fn from(axis: Axis) -> Self {
        let g = axis.graduation;

        space::Axis::new(
            &axis.measurement_unit,
            axis.unit_vector,
            g.set.into(),
            g.minimum,
            g.maximum,
            g.steps,
        )
        .unwrap_or_else(|e| panic!("Unable to create Axis as defined: {}", e))
    }
}

impl From<&space::Axis> for Axis {
    fn from(axis: &space::Axis) -> Self {
        Axis {
            measurement_unit: axis.measurement_unit(),
            graduation: axis.graduation().into(),
            unit_vector: axis.unit_vector().into(),
        }
    }
}

impl From<&Space> for space::Space {
    fn from(space: &Space) -> Self {
        let axes = space
            .axes
            .iter()
            .map(|a| a.clone().into())
            .collect::<Vec<_>>();

        let system = space::CoordinateSystem::new(space.origin.clone(), axes);

        space::Space::new(&space.name, system)
    }
}

impl From<&space::Space> for Space {
    fn from(space: &space::Space) -> Self {
        let axes = space.axes().iter().map(|a| a.into()).collect::<Vec<_>>();

        Space {
            name: space.name().clone(),
            origin: space.origin().clone().into(),
            axes,
        }
    }
}

pub fn to_spatial_objects(db: &DataBase, list: Vec<SpaceObject>) -> Vec<SpatialObject> {
    // Filter per Properties, in order to regroup by it, then build a single SpatialObject per Properties.
    let mut properties = HashMap::new();
    for object in list {
        let k = object.value.id().clone();
        properties.entry(k).or_insert_with(|| vec![]).push(object);
    }

    let mut results = vec![];
    for (k, v) in properties.iter() {
        // Group by spaces, to collect points shapes together
        let shapes = v
            .iter()
            .filter_map(|o| match db.space(&o.space_id) {
                Err(_) => None,
                Ok(space) => {
                    if let Ok(vertices) = space.decode(&o.position) {
                        Some(Shape {
                            type_name: "Point".to_string(),
                            reference_space: o.space_id.clone(),
                            vertices: vec![vertices],
                        })
                    } else {
                        None
                    }
                }
            })
            .collect();

        results.push(SpatialObject {
            properties: Properties {
                type_name: "Feature".to_string(),
                id: k.to_string(),
            },
            shapes,
        });
    }

    results
}

pub fn build_index(
    name: &str,
    version: &str,
    spaces: &[space::Space],
    objects: &[SpatialObject],
    scales: Option<Vec<Vec<u32>>>,
    max_elements: Option<usize>,
) -> Core {
    let mut properties = vec![];
    let mut space_set_objects = vec![];

    let mut properties_ref = vec![];

    {
        let mut properties_hm = HashMap::new();

        for object in objects {
            let value = match properties_hm.get(object.properties.id.as_str()) {
                Some(_) => {
                    properties_ref.push(object.properties.id.as_str());
                    properties_ref.len() - 1
                }
                None => {
                    properties_hm.insert(
                        object.properties.id.as_str(),
                        database::Properties::Feature(object.properties.id.clone()),
                    );

                    properties_ref.push(object.properties.id.as_str());
                    properties_ref.len() - 1
                }
            };

            for point in &object.shapes {
                assert_eq!(point.type_name, "Point");

                space_set_objects.push(SpaceSetObject::new(
                    &point.reference_space,
                    point.vertices[0].clone().into(),
                    value.into(),
                ))
            }
        }

        properties.append(&mut properties_hm.drain().map(|(_, v)| v).collect::<Vec<_>>());
    }

    properties.sort_unstable_by_key(|p| p.id().clone());

    space_set_objects.iter_mut().for_each(|object| {
        let id = properties_ref[object.value().u64() as usize];
        let value = properties.binary_search_by_key(&id, |p| p.id()).unwrap();
        object.set_value(value.into());
    });

    Core::new(
        name,
        version,
        spaces,
        properties,
        space_set_objects,
        scales,
        max_elements,
    )
}
