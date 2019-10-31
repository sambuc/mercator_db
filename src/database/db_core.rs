use super::space::Position;
use super::space::Shape;
use super::space::Space;
use super::space_db::SpaceDB;
use super::space_index::SpaceSetObject;
use super::DataBase;
use super::ResultSet;

pub struct CoreQueryParameters<'a> {
    pub db: &'a DataBase,
    pub output_space: Option<&'a str>,
    pub threshold_volume: Option<f64>,
    pub view_port: &'a Option<(Vec<f64>, Vec<f64>)>,
    pub resolution: &'a Option<Vec<u32>>,
}

impl CoreQueryParameters<'_> {
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

// FIXME: Ids are expected unique, irrespective of the enum variant!
#[derive(Clone, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
pub enum Properties {
    Feature(String),
    Unknown(String, String),
}

impl Properties {
    pub fn id(&self) -> &String {
        match self {
            Properties::Feature(id) => id,
            Properties::Unknown(id, _) => id,
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            Properties::Feature(_) => "Feature",
            Properties::Unknown(_, type_name) => type_name,
        }
    }

    pub fn feature<S>(id: S) -> Properties
    where
        S: Into<String>,
    {
        Properties::Feature(id.into())
    }

    pub fn unknown<S>(id: S, type_name: S) -> Properties
    where
        S: Into<String>,
    {
        Properties::Unknown(id.into(), type_name.into())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Core {
    title: String,
    version: String,
    properties: Vec<Properties>,
    space_db: Vec<SpaceDB>,
}

impl Core {
    pub fn new<S>(
        title: S,
        version: S,
        spaces: &[Space],
        properties: Vec<Properties>,
        space_objects: Vec<SpaceSetObject>,
        scales: Option<Vec<Vec<u32>>>,
        max_elements: Option<usize>,
    ) -> Self
    //Result<Self, String>
    where
        S: Into<String>,
    {
        // Sort out the space, and create a SpaceDB per reference space
        let mut space_dbs = vec![];

        for space in spaces {
            // Filter the points of this space, and encode them before creating the index.
            let filtered = space_objects
                .iter()
                .filter_map(|object| {
                    if object.space_id() == space.name() {
                        let position: Vec<f64> = object.position().into();
                        Some(SpaceSetObject::new(
                            space.name(),
                            space.encode(&position).unwrap(),
                            *object.value(),
                        ))
                    } else {
                        None
                    }
                })
                .collect();

            space_dbs.push(SpaceDB::new(&space, filtered, scales.clone(), max_elements))
        }

        Core {
            title: title.into(),
            version: version.into(),
            properties,
            space_db: space_dbs,
        }
    }

    // Check if the given space_id is referenced in the current core.
    pub fn is_empty(&self, space_id: &str) -> bool {
        for s in &self.space_db {
            if s.name() == space_id {
                return s.is_empty();
            }
        }

        // Not found, so the space is empty.
        true
    }

    pub fn name(&self) -> &String {
        &self.title
    }

    pub fn version(&self) -> &String {
        &self.version
    }

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
                    .decode(&Space::change_base(&position, space, unified)?)?
                    .into();
            }
        } else {
            // Decode the positions into f64 values, which are defined in their
            // respective reference space.
            for (position, _) in list {
                // Simply decode
                *position = space.decode(&position)?.into();
            }
        }

        Ok(())
    }

    // Search by positions defining a volume.
    // Positions ARE DEFINED IN F64 VALUES IN THE SPACE. NOT ENCODED!
    pub fn get_by_positions(
        &self,
        parameters: &CoreQueryParameters,
        positions: &[Position],
        from: &str,
    ) -> ResultSet {
        let CoreQueryParameters {
            db, output_space, ..
        } = parameters;

        let mut results = vec![];
        let count = positions.len();
        let from = db.space(from)?;

        // Filter positions based on the view port, if present
        let filtered = match parameters.view_port(from) {
            None => positions.iter().map(|p| p).collect::<Vec<_>>(),
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
                .map(|(position, fields)| (position, &self.properties[fields.value().as_usize()]))
                .collect::<Vec<_>>();
            Self::decode_positions(r.as_mut_slice(), to, db, output_space)?;

            results.push((s.name(), r));
        }

        Ok(results)
    }

    // Search by shape defining a volume:
    // * Hyperrectangle (MBB),
    // * HyperSphere (radius around a point),
    // * Point (Specific position)

    // SHAPE IS DEFINED IN F64 VALUES IN THE SPACE. NOT ENCODED!
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
                .map(|(position, fields)| (position, &self.properties[fields.value().as_usize()]))
                .collect::<Vec<_>>();
            Self::decode_positions(r.as_mut_slice(), current_space, db, output_space)?;

            results.push((s.name(), r));
        }

        Ok(results)
    }

    // Search by Id, a.k.a values
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
            .binary_search_by_key(&&id, |properties| properties.id())
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

    // Search by Label, a.k.a within a volume defined by the positions of an Id.
    // FIXME: NEED TO KEEP TRACK OF SPACE IDS AND DO CONVERSIONS
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
            .binary_search_by_key(&&id, |properties| properties.id())
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
                .flat_map(|v| v);

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
                    .map(|(position, fields)| {
                        (position, &self.properties[fields.value().as_usize()])
                    })
                    .collect::<Vec<_>>();

                Self::decode_positions(r.as_mut_slice(), to, db, output_space)?;

                results.push((s.name(), r));
            }
        }

        Ok(results)
    }
}
