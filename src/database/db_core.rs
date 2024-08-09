use serde::Deserialize;
use serde::Serialize;

use super::space::Position;
use super::space::Shape;
use super::space::Space;
use super::space_db::SpaceDB;
use super::space_index::SpaceSetObject;
use super::DataBase;
use super::IterObjects;
use super::IterPositions;
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

    fn decode_positions<'b>(
        list: IterObjects<'b>,
        space: &'b Space,
        db: &'b DataBase,
        output_space: &Option<&str>,
    ) -> Result<IterObjects<'b>, String> {
        let b: IterObjects = if let Some(unified_id) = *output_space {
            let unified = db.space(unified_id)?;

            // Rebase the point to the requested output space before decoding.
            Box::new(list.filter_map(move |(position, properties)| {
                match Space::change_base(&position, space, unified) {
                    Err(_) => None,
                    Ok(rebased) => match unified.decode(&rebased) {
                        Err(_) => None,
                        Ok(decoded) => Some((decoded.into(), properties)),
                    },
                }
            }))
        } else {
            // Decode the positions into f64 values, which are defined in their
            // respective reference space.
            Box::new(list.filter_map(
                move |(position, properties)| match space.decode(&position) {
                    Err(_) => None,
                    Ok(decoded) => Some((decoded.into(), properties)),
                },
            ))
        };

        Ok(b)
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
    pub fn get_by_positions<'d>(
        &'d self,
        parameters: &'d CoreQueryParameters,
        positions: Vec<Position>,
        space_id: &'d str,
    ) -> ResultSet<'d> {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let mut results = vec![];
        let from = db.space(space_id)?;

        for s in &self.space_db {
            let to = db.space(s.name())?;

            // Filter positions based on the view port, if present
            // FIXME: remove clone() on positions?
            let filtered: IterPositions = match parameters.view_port(from) {
                None => Box::new(positions.clone().into_iter()),
                Some(view_port) => Box::new(
                    positions
                        .clone()
                        .into_iter()
                        .filter(move |p| view_port.contains(p)),
                ),
            };

            // Rebase the positions into the current space
            let p = filtered.filter_map(move |position| {
                match Space::change_base(&position, from, to) {
                    Err(_) => None,
                    Ok(position) => {
                        let position: Vec<f64> = position.into();
                        match to.encode(&position) {
                            Err(_) => None,
                            Ok(position) => Some(position),
                        }
                    }
                }
            });

            // Select the data based on the rebased viewport filter.
            let r = s
                .get_by_positions(p, parameters)?
                .map(move |(position, fields)| (position, &self.properties[fields.value()]));

            results.push((
                s.name(),
                Self::decode_positions(Box::new(r), to, db, output_space)?,
            ));
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
    pub fn get_by_shape<'d>(
        &'d self,
        parameters: &'d CoreQueryParameters,
        shape: Shape,
        space_id: &'d str,
    ) -> ResultSet<'d> {
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

            let r = s
                .get_by_shape(current_shape, parameters)?
                .map(move |(position, fields)| (position, &self.properties[fields.value()]));

            results.push((
                s.name(),
                Self::decode_positions(Box::new(r), current_space, db, output_space)?,
            ));
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
    pub fn get_by_id<'s, S>(
        &'s self,
        parameters: &'s CoreQueryParameters,
        id: S,
    ) -> Result<Vec<(&String, IterPositions<'s>)>, String>
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

                let positions_by_id = s.get_by_id(offset, parameters)?;

                //Self::decode_positions(r.as_mut_slice(), current_space, db, output_space)?;
                let positions: IterPositions = if let Some(unified_id) = *output_space {
                    let unified = db.space(unified_id)?;

                    // Rebase the point to the requested output space before decoding.
                    Box::new(positions_by_id.filter_map(move |position| {
                        match Space::change_base(&position, current_space, unified) {
                            Err(_) => None,
                            Ok(rebased) => match unified.decode(&rebased) {
                                Err(_) => None,
                                Ok(decoded) => Some(decoded.into()),
                            },
                        }
                    }))
                } else {
                    // Decode the positions into f64 values, which are defined in their
                    // respective reference space.
                    Box::new(positions_by_id.filter_map(move |position| {
                        match current_space.decode(&position) {
                            Err(_) => None,
                            Ok(decoded) => Some(decoded.into()),
                        }
                    }))
                };

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
    pub fn get_by_label<'d, S>(
        &'d self,
        parameters: &'d CoreQueryParameters,
        id: S,
    ) -> ResultSet<'d>
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
                .filter_map(move |s| {
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

            // Select based on the volume, and filter out the label position themselves.
            for s in &self.space_db {
                let to = db.space(s.name())?;

                let search_volume: IterPositions = if let Some(view) = view_port.clone() {
                    Box::new(search_volume.clone().filter(move |p| view.contains(p)))
                } else {
                    Box::new(search_volume.clone())
                };

                // Convert the search Volume into the target space.
                let p = search_volume.filter_map(move |position| {
                    match Space::change_base(&position, Space::universe(), to) {
                        Err(_) => None,
                        Ok(position) => Some(position),
                    }
                });

                let r = s
                    .get_by_positions(p, parameters)?
                    .filter_map(move |(position, fields)| {
                        if fields.value() == offset {
                            None
                        } else {
                            Some((position, &self.properties[fields.value()]))
                        }
                    });

                results.push((
                    s.name(),
                    Self::decode_positions(Box::new(r), to, db, output_space)?,
                ));
            }
        }

        Ok(results)
    }
}
