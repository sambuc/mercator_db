use serde::Deserialize;
use serde::Serialize;

use super::space::Position;
use super::space::Shape;
use super::space::Space;
use super::space_db::SpaceDB;
use super::space_index::SpaceSetObject;
use super::DataBase;
use super::ResultSet;

/// Query Parameters.
pub struct CoreQueryParameters<'a> {
    /// Database to use.
    pub db: &'a DataBase,
    /// Output reference space into which to convert results.
    pub output_space: Option<&'a str>,
    /// Volume value to use to select the index resolution.
    //FIXME: IS this necessary given view_port?
    pub threshold_volume: Option<f64>,
    /// Full definition of the view port, a.k.a the volume being
    /// displayed.
    pub view_port: &'a Option<(Vec<f64>, Vec<f64>)>,
    /// Index resolution to use.
    pub resolution: &'a Option<Vec<u32>>,
}

impl CoreQueryParameters<'_> {
    /// Build a minimum bounding box out of the provided viewport, and
    /// rebase it in the target space.
    ///
    /// # Parameters
    ///
    ///  * `space`:
    ///      Space to use for the encoded coordinates of the minimum
    ///      bounding box.
    pub fn view_port(&self, space: &Space) -> Option<Shape> {
        if let Some((low, high)) = self.view_port {
            let view_port = Shape::BoundingBox(low.into(), high.into());
            match view_port.rebase(Space::universe(), space) {
                Err(_) => None,
                Ok(view) => Some(view),
            }
        } else {
            None
        }
    }
}

/// Definition of the volumetric objects identifiers.
///
/// We have two parts to it, first the *kind* and the actual, *id* used
/// to distinguish different objects.
// FIXME: Ids are expected unique, irrespective of the enum variant!
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Properties {
    /// Spatial Features.
    Feature(String),
    /// Unoptimized arbitrary kind of *identifiers*.
    Unknown(String, String),
}

impl Properties {
    /// Extract the *identifier* of this spatial object.
    pub fn id(&self) -> &str {
        match self {
            Properties::Feature(id) => id,
            Properties::Unknown(id, _) => id,
        }
    }

    /// Extract the *kind* of spatial object.
    pub fn type_name(&self) -> &str {
        match self {
            Properties::Feature(_) => "Feature",
            Properties::Unknown(_, type_name) => type_name,
        }
    }

    /// Instantiate a new *feature*.
    ///
    /// # Parameters
    ///
    ///  * `id`:
    ///      The identifier of the object, which can be converted into a
    ///      `String`.
    pub fn feature<S>(id: S) -> Properties
    where
        S: Into<String>,
    {
        Properties::Feature(id.into())
    }

    /// Instantiate a new arbitrary kind of object, with the given id.
    ///
    /// # Parameters
    ///
    ///  * `id`:
    ///      The identifier of the object, which can be converted into a
    ///      `String`.
    ///
    ///  * `type_name`:
    ///      A value which can be converted into a `String`, and
    ///      represent the **kind** of the object.
    pub fn unknown<S>(id: S, type_name: S) -> Properties
    where
        S: Into<String>,
    {
        Properties::Unknown(id.into(), type_name.into())
    }
}

/// Index over a single dataset
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Core {
    title: String,
    version: String,
    properties: Vec<Properties>,
    space_db: Vec<SpaceDB>,
}

impl Core {
    /// Instantiate a new index for a dataset.
    ///
    /// # Parameters
    ///
    ///  * `title`:
    ///     The title to use for the new dataset.
    ///
    ///  * `version`:
    ///     The revision of the new dataset.
    ///
    ///  * `spaces`:
    ///     The list of reference spaces used within the dataset.
    ///
    ///  * `properties`:
    ///     The *identifiers*, has an ordered list, which is referenced
    ///     by the `space_objects` by offset within this list.
    ///
    ///  * `space_objects`:
    ///     A list of links between volumetric positions and
    ///     identifiers.
    ///
    ///  * `scales`:
    ///     A list of resolutions for which to build indices. Each value
    ///     represent the number of bits of precision to **remove** from
    ///     the coordinates to build the index.
    ///
    ///  * `max_elements`:
    ///     The minimum number of positions to use as a stopping
    ///     condition while building automatically multiple resolutions
    ///     of the index.
    ///
    ///     Each consecutive index will contains at most half the number
    ///     of data points than the next finer-grained index.
    ///
    ///     The minimum number of elements contained within an index is
    ///     this value or the number of *identifiers*, whichever is
    ///     greater.
    pub fn new<S>(
        title: S,
        version: S,
        spaces: &[Space],
        properties: Vec<Properties>,
        space_objects: Vec<SpaceSetObject>,
        scales: Option<Vec<Vec<u32>>>,
        max_elements: Option<usize>,
    ) -> Result<Self, String>
    where
        S: Into<String>,
    {
        // Sort out the space, and create a SpaceDB per reference space
        let mut space_dbs = vec![];

        // We cannot return less that the total number of individual Ids stored
        // in the index for a full-volume query.
        let max_elements = max_elements.map(|elem| elem.max(properties.len()));

        for space in spaces {
            // Filter the points of this space, and encode them before creating the index.
            let mut filtered = space_objects
                .iter()
                .filter(|object| object.space_id() == space.name())
                // Clone only the selected objects, not all of them!
                .cloned()
                .collect::<Vec<_>>();

            for object in filtered.iter_mut() {
                let position: Vec<f64> = object.position().into();
                object.set_position(space.encode(&position)?);
            }

            space_dbs.push(SpaceDB::new(space, filtered, scales.clone(), max_elements))
        }

        Ok(Core {
            title: title.into(),
            version: version.into(),
            properties,
            space_db: space_dbs,
        })
    }

    /// Title of the dataset.
    pub fn name(&self) -> &String {
        &self.title
    }

    /// Revision of the dataset.
    pub fn version(&self) -> &String {
        &self.version
    }

    /// List of *identifiers* contained in this dataset.
    pub fn keys(&self) -> &Vec<Properties> {
        &self.properties
    }

    fn decode_positions(
        list: &mut [(Position, &Properties)],
        space: &Space,
        db: &DataBase,
        output_space: &Option<&str>,
    ) -> Result<(), String> {
        if let Some(unified_id) = *output_space {
            let unified = db.space(unified_id)?;

            // Rebase the point to the requested output space before decoding.
            for (position, _) in list {
                *position = unified
                    .decode(&Space::change_base(position, space, unified)?)?
                    .into();
            }
        } else {
            // Decode the positions into f64 values, which are defined in their
            // respective reference space.
            for (position, _) in list {
                // Simply decode
                *position = space.decode(position)?.into();
            }
        }

        Ok(())
    }

    /// Retrieve everything located at specific positions.
    ///
    /// # Parameters
    ///
    ///  * `parameters`:
    ///     Search parameters, see [CoreQueryParameters](struct.CoreQueryParameters.html).
    ///
    ///  * `positions`:
    ///     Volume to use to filter data points.
    ///
    ///  * `space_id`:
    ///     *positions* are defined as decoded coordinates in this
    ///     reference space.
    ///
    /// [shape]: space/enum.Shape.html
    pub fn get_by_positions(
        &self,
        parameters: &CoreQueryParameters,
        positions: &[Position],
        space_id: &str,
    ) -> ResultSet {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let mut results = vec![];
        let count = positions.len();
        let from = db.space(space_id)?;

        // Filter positions based on the view port, if present
        let filtered = match parameters.view_port(from) {
            None => positions.iter().collect::<Vec<_>>(),
            Some(view_port) => positions
                .iter()
                .filter(|&p| view_port.contains(p))
                .collect::<Vec<_>>(),
        };

        for s in &self.space_db {
            let to = db.space(s.name())?;
            let mut p = Vec::with_capacity(count);

            for position in filtered.as_slice() {
                let position: Vec<f64> = Space::change_base(position, from, to)?.into();
                p.push(to.encode(&position)?);
            }

            let mut r = s
                .get_by_positions(&p, parameters)?
                .into_iter()
                .map(|(position, fields)| (position, &self.properties[fields.value()]))
                .collect::<Vec<_>>();
            Self::decode_positions(r.as_mut_slice(), to, db, output_space)?;

            results.push((s.name(), r));
        }

        Ok(results)
    }

    /// Search using a [shape] which defines a volume.
    ///
    /// # Parameters
    ///
    ///  * `parameters`:
    ///     Search parameters, see [CoreQueryParameters](struct.CoreQueryParameters.html).
    ///
    ///  * `shape`:
    ///     Volume to use to filter data points.
    ///
    ///  * `space_id`:
    ///     *shape* is defined as decoded coordinates in this
    ///     reference space.
    ///
    /// [shape]: space/enum.Shape.html
    pub fn get_by_shape(
        &self,
        parameters: &CoreQueryParameters,
        shape: &Shape,
        space_id: &str,
    ) -> ResultSet {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let mut results = vec![];
        let shape_space = db.space(space_id)?;

        for s in &self.space_db {
            let current_space = db.space(s.name())?;

            let current_shape = shape.rebase(shape_space, current_space)?;
            //            println!("current shape: {:?}", current_shape);
            //            let current_shape = shape.encode(current_space)?;
            //            println!("current shape Encoded: {:?}", current_shape);

            let mut r = s
                .get_by_shape(&current_shape, parameters)?
                .into_iter()
                .map(|(position, fields)| (position, &self.properties[fields.value()]))
                .collect::<Vec<_>>();
            Self::decode_positions(r.as_mut_slice(), current_space, db, output_space)?;

            results.push((s.name(), r));
        }

        Ok(results)
    }

    /// Search by Id, a.k.a retrieve all the positions linked to this id.
    ///
    /// # Parameters
    ///
    ///  * `parameters`:
    ///     Search parameters, see [CoreQueryParameters](struct.CoreQueryParameters.html).
    ///
    ///  * `id`:
    ///     Identifier for which to retrieve is positions.
    ///
    pub fn get_by_id<S>(
        &self,
        parameters: &CoreQueryParameters,
        id: S,
    ) -> Result<Vec<(&String, Vec<Position>)>, String>
    where
        S: Into<String>,
    {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let id: String = id.into();
        let mut results = vec![];

        // Do we have this ID registered at all?
        if let Ok(offset) = self
            .properties
            .binary_search_by_key(&id.as_str(), |properties| properties.id())
        {
            // Yes, so now let's find all the position linked to it, per
            // reference space
            for s in &self.space_db {
                let current_space = db.space(s.name())?;

                let mut positions = s.get_by_id(offset, parameters)?;

                //Self::decode_positions(r.as_mut_slice(), current_space, db, output_space)?;
                if let Some(unified_id) = *output_space {
                    let unified = db.space(unified_id)?;

                    // Rebase the point to the requested output space before decoding.
                    for position in &mut positions {
                        *position = unified
                            .decode(&Space::change_base(position, current_space, unified)?)?
                            .into();
                    }
                } else {
                    // Decode the positions into f64 values, which are defined in their
                    // respective reference space.
                    for position in &mut positions {
                        // Simply decode
                        *position = current_space.decode(position)?.into();
                    }
                }

                results.push((s.name(), positions));
            }
        }

        Ok(results)
    }

    /// Search by label, a.k.a use an identifier to define the search
    /// volume.
    ///
    /// # Parameters
    ///
    ///  * `parameters`:
    ///     Search parameters, see [CoreQueryParameters](struct.CoreQueryParameters.html).
    ///
    ///  * `id`:
    ///     Identifier to use to define the search volume.
    ///
    pub fn get_by_label<S>(&self, parameters: &CoreQueryParameters, id: S) -> ResultSet
    where
        S: Into<String>,
    {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let id: String = id.into();
        let mut results = vec![];

        // Convert the view port to the encoded space coordinates
        let view_port = parameters.view_port(Space::universe());

        if let Ok(offset) = self
            .properties
            .binary_search_by_key(&id.as_str(), |properties| properties.id())
        {
            // Generate the search volume. Iterate over all reference spaces, to
            // retrieve a list of SpaceSetObjects linked to `id`, then iterate
            // over the result to generate a list of positions in Universe.
            let search_volume = self
                .space_db
                .iter()
                .filter_map(|s| {
                    match db.space(s.name()) {
                        Err(_) => None,
                        Ok(from) => match s.get_by_id(offset, parameters) {
                            Err(_) => None,
                            Ok(v) => {
                                // Convert the search Volume into Universe.
                                let mut p = vec![];
                                for position in v {
                                    if let Ok(position) =
                                        Space::change_base(&position, from, Space::universe())
                                    {
                                        p.push(position)
                                    }
                                }

                                Some(p)
                            }
                        },
                    }
                })
                .flatten();

            let search_volume = if let Some(view) = view_port {
                search_volume
                    .filter(|p| view.contains(p))
                    .collect::<Vec<_>>()
            } else {
                search_volume.collect::<Vec<_>>()
            };

            // Select based on the volume, and filter out the label position themselves.
            for s in &self.space_db {
                let to = db.space(s.name())?;
                let mut p = vec![];

                // Convert the search Volume into the target space.
                for position in &search_volume {
                    let position = Space::change_base(position, Space::universe(), to)?;
                    p.push(position);
                }

                let mut r = s
                    .get_by_positions(&p, parameters)?
                    .into_iter()
                    .filter_map(|(position, fields)| {
                        if fields.value() == offset {
                            None
                        } else {
                            Some((position, &self.properties[fields.value()]))
                        }
                    })
                    .collect::<Vec<_>>();

                Self::decode_positions(r.as_mut_slice(), to, db, output_space)?;

                results.push((s.name(), r));
            }
        }

        Ok(results)
    }
}
