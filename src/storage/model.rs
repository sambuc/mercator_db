//! Model definitions for serialisation.
//!
//! The following definitions are used as part of the serialisation
//! process to exchange objects either through network or to storage.

use std::collections::HashMap;

use serde::Deserialize;
use serde::Serialize;

use crate::database;
use database::space;
use database::space_index::SpaceSetObject;
use database::Core;

/// Reference space definition.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Space {
    /// **Id** of the space.
    pub name: String,

    /// Position of the origin of axis expressed in Universe coordinates.
    pub origin: Vec<f64>,

    /// List of axes of the space.
    pub axes: Vec<Axis>,
}

/// Reference space axis definition.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Axis {
    /// Length unit for the value `1.0`.
    pub measurement_unit: String,

    /// Define the valid range of number on this axis.
    pub graduation: Graduation,

    /// Vector which defines the direction of the axis in the Universe
    pub unit_vector: Vec<f64>,
}

/// Valid range of numbers on the axis.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Graduation {
    /// Mathematical Number Set of numbers allowed.
    pub set: String,

    /// Minimum value allowed, included.
    pub minimum: f64,

    /// Maximum value allowed, excluded.
    pub maximum: f64,

    /// Number of distinct positions between `[min; max[`
    pub steps: u64,
}

/// A single spatial location.
///
/// This has a value per dimension of the space it is expressed in.
pub type Point = Vec<f64>;

pub mod v1 {
    //! REST API objects, v1.

    use std::collections::HashMap;

    use serde::Deserialize;
    use serde::Serialize;

    use crate::database;
    use database::space;

    use super::Point;
    use super::Properties;

    /// Links Properties to a list of spatial volumes.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SpatialObject {
        /// Definition of the `properties` to tag in space.
        pub properties: Properties,

        /// List of volumes associated with `properties`.
        pub shapes: Vec<Shape>,
    }

    /// Define a Shape, within a specific reference space.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Shape {
        /// Type of the shape, which is used to interpret the list of `vertices`.
        #[serde(rename = "type")]
        pub type_name: String,

        /// Id of the reference space the points are defined in.
        #[serde(rename = "space")]
        pub reference_space: String,

        /// List of spatial positions.
        pub vertices: Vec<Point>,
    }

    /// Convert a list of properties grouped by space id, then positions to a
    /// list of Spatial Objects for the rest API v1.
    ///
    /// # Parameters
    ///
    ///  * `list`:
    ///      A list of (**Space Id**, [ ( *Spatial position*, `&Properties` ) ]) tuples.
    pub fn to_spatial_objects(
        list: Vec<(&String, Vec<(space::Position, &database::Properties)>)>,
    ) -> Vec<SpatialObject> {
        // Filter per Properties, in order to regroup by it, then build a single SpatialObject per Properties.
        let mut hashmap = HashMap::new();
        for (space, v) in list {
            for (position, properties) in v {
                hashmap
                    .entry(properties)
                    .or_insert_with(|| vec![])
                    .push((space, position));
            }
        }

        let mut results = vec![];
        for (properties, v) in hashmap.iter() {
            // Group by spaces, to collect points shapes together
            let shapes = v
                .iter()
                .map(|(space_id, position)| Shape {
                    type_name: "Point".to_string(),
                    reference_space: (*space_id).clone(),
                    vertices: vec![position.into()],
                })
                .collect();

            results.push(SpatialObject {
                properties: properties.into(),
                shapes,
            });
        }

        results
    }
}

pub mod v2 {
    //! REST API objects, v2.

    use std::collections::HashMap;

    use serde::Deserialize;
    use serde::Serialize;

    use crate::database;
    use database::space;

    use super::Point;
    use super::Properties;

    /// Links Properties to a list of spatial volumes.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct SpatialObject {
        /// Definition of the `properties` to tag in space.
        pub properties: Properties,

        /// List of volumes associated with `properties`.
        pub volumes: Vec<Volume>,
    }

    /// Defines a volume as the union of geometric shapes.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    pub struct Volume {
        /// Reference space id.
        pub space: String,

        /// List of geometric shapes defined in the reference space
        /// `space`.
        pub shapes: Vec<Shape>,
    }

    /// Describes an homogeneous list of geometric shapes.
    #[derive(Clone, Debug, Deserialize, Serialize)]
    #[serde(rename_all = "lowercase")]
    pub enum Shape {
        /// List of points.
        Points(Vec<Point>),

        /// List of Bounding boxes or *hyper rectangles* for which each
        /// face is perpendicular to one of the axis of the reference
        /// space.
        ///
        /// That property allows us to describe such a hyperrectangle
        /// with two corners:
        ///
        ///  * one for which all the coordinates are the smallest among
        ///    all the corners, per dimension, which is called here
        ///    *lower corner*
        ///
        ///  * one for which all the coordinates are the greatest among
        ///    all the corners, per dimension, which is called
        ///    *higher corner*.
        ///
        /// The list simply stores tuples of (`lower corner`,
        /// `higher corner`), as this is enough to reconstruct all the
        /// corners of the bounding box.
        BoundingBoxes(Vec<(Point, Point)>),

        /// List of hyperspheres, stored as (`center`, radius) tuples.
        HyperSpheres(Vec<(Point, f64)>),
    }

    /// Convert a list of properties grouped by space id, then positions to a
    /// list of Spatial Objects for the rest API v2.
    ///
    /// # Parameters
    ///
    ///  * `list`:
    ///      A list of (**Space Id**, [ ( *Spatial position*, `&Properties` ) ]) tuples.
    pub fn to_spatial_objects(
        list: Vec<(&String, Vec<(space::Position, &database::Properties)>)>,
    ) -> Vec<SpatialObject> {
        // Filter per Properties, in order to regroup by it, then build a single SpatialObject per Properties.
        let mut hashmap = HashMap::new();
        for (space, v) in list {
            for (position, properties) in v {
                hashmap
                    .entry(properties)
                    .or_insert_with(HashMap::new)
                    .entry(space)
                    .or_insert_with(|| vec![])
                    .push(position.into());
            }
        }

        let mut results = vec![];
        for (properties, v) in hashmap.iter_mut() {
            let volumes = v
                .drain()
                .map(|(space, positions)| Volume {
                    space: space.clone(),
                    shapes: vec![Shape::Points(positions)],
                })
                .collect();

            results.push(SpatialObject {
                properties: properties.into(),
                volumes,
            });
        }

        results
    }
}

/// **Properties** which are registered at one or more spatial locations.
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Properties {
    /// The **type** of *Id*, this allows for different kinds of objects
    /// to have the same *Id*, but handled distinctly.
    #[serde(rename = "type")]
    pub type_name: String,

    /// An arbitrary string.
    pub id: String,
}

impl From<&space::Graduation> for Graduation {
    fn from(g: &space::Graduation) -> Self {
        Graduation {
            set: (&g.set).into(),
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
            g.set.as_str().into(),
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
            measurement_unit: axis.measurement_unit().into(),
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
            origin: space.origin().into(),
            axes,
        }
    }
}

impl From<&&database::Properties> for Properties {
    fn from(p: &&database::Properties) -> Self {
        Properties {
            type_name: p.type_name().to_string(),
            id: p.id().into(),
        }
    }
}

pub use v1::SpatialObject;

/// Generate an index.
///
/// # Parameters
///
/// * `name`:
///     Name to give to the index.
///
/// * `version`:
///     Parameter to distinguish revisions of an index.
///
/// * `spaces`:
///     A list of the reference spaces. Only objects whose reference
///     space is known will be indexed.
///
/// * `objects`:
///     The data points to index.
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
pub fn build_index(
    name: &str,
    version: &str,
    spaces: &[space::Space],
    objects: &[SpatialObject],
    scales: Option<Vec<Vec<u32>>>,
    max_elements: Option<usize>,
) -> Result<Core, String> {
    let mut properties = vec![];
    let mut space_set_objects = vec![];
    {
        let mut properties_ref = vec![];
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
                    // Use a reference to prevent an allocation
                    (&point.vertices[0]).into(),
                    value,
                ))
            }
        }

        properties.append(&mut properties_hm.drain().map(|(_, v)| v).collect::<Vec<_>>());
        properties.sort_unstable_by(|a, b| a.id().cmp(b.id()));

        space_set_objects.iter_mut().for_each(|object| {
            let id = properties_ref[object.value()];
            let value = properties.binary_search_by_key(&id, |p| p.id()).unwrap();
            object.set_value(value);
        });
    }

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
