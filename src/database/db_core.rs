use super::space::Position;
use super::space::Shape;
use super::space::Space;
use super::space_db::SpaceDB;
use super::space_index::SpaceSetObject;
use super::DataBase;
use super::ResultSet;
use crate::SpaceObject;

pub struct CoreQueryParameters<'a> {
    pub db: &'a DataBase,
    pub output_space: Option<&'a str>,
    pub threshold_volume: Option<f64>,
    pub resolution: Option<Vec<u64>>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
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

impl PartialEq for Properties {
    fn eq(&self, other: &Self) -> bool {
        self.id() == other.id() && self.type_name() == other.type_name()
    }
}

impl Eq for Properties {}

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
                    if &object.space_id().0 == space.name() {
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

            space_dbs.push(SpaceDB::new(space.name(), filtered))
        }

        Core {
            title: title.into(),
            version: version.into(),
            properties,
            space_db: space_dbs,
        }
    }

    // Check if the given space_id is referenced in the current core.
    pub fn is_empty<S>(&self, space_id: S) -> bool
    where
        S: Into<String>,
    {
        let id = space_id.into();
        for s in &self.space_db {
            if s.name() == &id {
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

    fn to_space_object(&self, space_id: &str, list: Vec<SpaceSetObject>) -> Vec<SpaceObject> {
        list.into_iter()
            .map(|o| {
                let offset: usize = o.value().into();
                let value = self.properties[offset].clone();
                SpaceObject {
                    space_id: space_id.to_string(),
                    position: o.position().clone(),
                    value,
                }
            })
            .collect()
    }

    fn decode_positions(
        list: &mut [SpaceObject],
        space: &Space,
        db: &DataBase,
        output_space: &Option<&str>,
    ) -> Result<(), String> {
        if let Some(unified_id) = *output_space {
            let unified = db.space(unified_id)?;

            // Rebase the point to the requested output space before decoding.
            for o in list {
                o.position = unified
                    .decode(&Space::change_base(&o.position, space, unified)?)?
                    .into();
                o.space_id = unified_id.to_string();
            }
        } else {
            // Decode the positions into f64 values, which are defined in their
            // respective reference space.
            for o in list {
                // Simply decode
                o.position = space.decode(&o.position)?.into();
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
            db,
            output_space,
            threshold_volume,
            resolution,
        } = parameters;

        let mut results = vec![];
        let count = positions.len();
        let from = db.space(from)?;

        for s in &self.space_db {
            let to = db.space(s.name())?;
            let mut p = Vec::with_capacity(count);

            for position in positions {
                let position: Vec<f64> = Space::change_base(position, from, to)?.into();
                p.push(to.encode(&position)?);
            }

            let r = s.get_by_positions(&p, threshold_volume, resolution)?;
            let mut r = self.to_space_object(s.name(), r);

            Self::decode_positions(&mut r, to, db, output_space)?;

            results.append(&mut r);
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
            db,
            output_space,
            threshold_volume,
            resolution,
        } = parameters;

        let mut results = vec![];
        let shape_space = db.space(space_id)?;

        for s in &self.space_db {
            let current_space = db.space(s.name())?;

            let current_shape = shape.rebase(shape_space, current_space)?;
            //            println!("current shape: {:?}", current_shape);
            //            let current_shape = shape.encode(current_space)?;
            //            println!("current shape Encoded: {:?}", current_shape);

            let r = s.get_by_shape(&current_shape, threshold_volume, resolution)?;
            let mut r = self.to_space_object(s.name(), r);

            Self::decode_positions(&mut r, current_space, db, output_space)?;

            results.append(&mut r);
        }

        Ok(results)
    }

    // Search by Id, a.k.a values
    pub fn get_by_id<S>(&self, parameters: &CoreQueryParameters, id: S) -> ResultSet
    where
        S: Into<String>,
    {
        let CoreQueryParameters {
            db,
            output_space,
            threshold_volume,
            resolution,
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

                let r = s.get_by_id(offset, threshold_volume, resolution)?;
                let mut r = self.to_space_object(s.name(), r);

                Self::decode_positions(&mut r, current_space, db, output_space)?;

                results.append(&mut r);
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
            db,
            output_space,
            threshold_volume,
            resolution,
        } = parameters;

        let id: String = id.into();
        let mut results = vec![];

        if let Ok(offset) = self
            .properties
            .binary_search_by_key(&&id, |properties| properties.id())
        {
            // Generate the search volume. Iterate over all reference spaces, to
            // retrieve a list of SpaceSetObjects linked to `id`, then iterate
            // over the result to generate a list of positions.
            let search_volume = self
                .space_db
                .iter()
                .filter_map(
                    |s| match s.get_by_id(offset, threshold_volume, resolution) {
                        Ok(v) => Some(v),
                        Err(_) => None,
                    },
                )
                .flat_map(|v| v)
                .map(|o| o.position().clone())
                .collect::<Vec<_>>();

            /*
                let search_volume = self
                .space_db
                .iter()
                .filter_map(|s| match s.get_by_id(offset, threshold_volume) {
                    Err(_) => None,
                    Ok(v) => Some((
                        s.name(),
                        v.into_iter().map(|o| o.position()).collect::<Vec<_>>(),
                    )),
                })
                .filter_map(|(space_id, list)| match db.space(space_id) {
                    Err(_) => None,
                    Ok(space) => Some((
                        space_id,
                        list.into_iter()
                            .map(|o| space.decode(o).into())
                            .collect::<Vec<Position>>(),
                    )),
                }).filter_map(|(space_id, list)|)
                .collect::<Vec<_>>();
            */

            // Select based on the volume, and filter out the label position themselves.
            for s in &self.space_db {
                let to = db.space(s.name())?;

                let r = s.get_by_positions(&search_volume, threshold_volume, resolution)?;
                let mut r = self.to_space_object(s.name(), r);

                Self::decode_positions(&mut r, to, db, output_space)?;

                results.append(&mut r);
            }
        }

        Ok(results)
    }
}

impl ironsea_index::Record<String> for Core {
    fn key(&self) -> String {
        self.title.clone()
    }
}
