mod db_core;
pub mod space;
mod space_db;
mod space_index;

use std::collections::HashMap;
use std::fs::File;
use std::hash::Hash;
use std::hash::Hasher;

use ironsea_index::Indexed;
use ironsea_table_vector::VectorTable;
use memmap::Mmap;

pub use db_core::Core;
pub use db_core::CoreQueryParameters;
pub use db_core::Properties;
use space::Position;
use space::Space;
pub use space_index::SpaceSetObject;

pub type ResultSet = Result<Vec<SpaceObject>, String>;
pub type ReferenceSpaceIndex = ironsea_index_hashmap::Index<VectorTable<Space>, Space, String>;
type CoreIndex = ironsea_index_hashmap::Index<VectorTable<Core>, Core, String>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceId(String);

impl SpaceId {
    pub fn new<S>(space_name: S) -> Self
    where
        S: Into<String>,
    {
        SpaceId(space_name.into())
    }

    pub fn get(&self, index: &ReferenceSpaceIndex) -> Self {
        let s = index.find(&self.0);
        assert_eq!(s.len(), 1);

        SpaceId(s[0].name().clone())
    }
}

impl<S> From<S> for SpaceId
where
    S: Into<String>,
{
    fn from(id: S) -> Self {
        SpaceId(id.into())
    }
}

impl PartialEq for SpaceId {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct SpaceObject {
    pub space_id: String,
    pub position: Position,
    pub value: Properties,
}

impl PartialEq for SpaceObject {
    fn eq(&self, other: &Self) -> bool {
        self.space_id == other.space_id
            && self.value == other.value
            && self.position == other.position
    }
}

impl Eq for SpaceObject {}

impl Hash for SpaceObject {
    //FIXME: ADD HASHING IMPLEMENTATION, REQUIRED FOR distinct()!
    fn hash<H: Hasher>(&self, state: &mut H) {
        unimplemented!()
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct DataBase {
    reference_spaces: ReferenceSpaceIndex,
    cores: CoreIndex,
}

impl DataBase {
    pub fn new(spaces: Vec<Space>, cores: Vec<Core>) -> Self {
        DataBase {
            reference_spaces: ReferenceSpaceIndex::new(VectorTable::new(spaces)),
            cores: CoreIndex::new(VectorTable::new(cores)),
        }
    }

    pub fn load(indices: &[&str]) -> Result<Self, String> {
        let mut spaces = HashMap::new();
        let mut cores = vec![];

        for index in indices.iter() {
            let (core_spaces, core) = DataBase::load_core(index)?;
            for core_space in core_spaces {
                if let Some(space) = spaces.get(core_space.name()) {
                    // Space is already registered, but with a different definitions.
                    if space != &core_space {
                        return Err(format!(
                            "Reference Space ID `{}` defined two times, but differently\n{:?}\n VS \n{:?}",
                            core_space.name(),
                            spaces.get(core_space.name()),
                            core_space
                        ));
                    }
                } else {
                    spaces.insert(core_space.name().clone(), core_space);
                }
            }

            cores.push(core);
        }

        let spaces = spaces.drain().map(|(_, v)| v).collect();

        Ok(DataBase::new(spaces, cores))
    }

    fn mmap_file(filename: &str) -> Result<Mmap, String> {
        let file_in = match File::open(filename) {
            Err(e) => return Err(format!("{:?}", e)),
            Ok(file) => file,
        };

        match unsafe { Mmap::map(&file_in) } {
            Err(e) => Err(format!("{:?}", e)),
            Ok(mmap) => Ok(mmap),
        }
    }

    pub fn load_core(name: &str) -> Result<(Vec<Space>, Core), String> {
        let fn_index = format!("{}.index", name);

        let mmap = DataBase::mmap_file(&fn_index)?;

        match bincode::deserialize(&mmap[..]) {
            Err(e) => Err(format!("Index deserialization error: {:?}", e)),
            Ok(index) => Ok(index),
        }
    }

    // Check if the given space_id is referenced in the DB.
    fn is_empty<S>(&self, id: S) -> bool
    where
        S: Into<String>,
    {
        let id = id.into();

        for s in self.cores.keys() {
            let core: &Core = self.cores.find(s)[0];
            if !core.is_empty(id.clone()) {
                return false;
            }
        }

        true
    }

    fn check_exactly_one<'t, T, S>(list: &[&'t T], name: S, value: S) -> Result<&'t T, String>
    where
        S: Into<String>,
    {
        if list.len() > 1 {
            Err(format!(
                "Multiple {} registered under `{}`: {}",
                name.into(),
                value.into(),
                list.len()
            ))
        } else if list.is_empty() {
            Err(format!(
                "No {} registered under `{}`: {}",
                name.into(),
                value.into(),
                list.len()
            ))
        } else {
            Ok(&list[0])
        }
    }

    pub fn space_id<S>(&self, name: S) -> Result<SpaceId, String>
    where
        S: Into<String>,
    {
        let name = name.into();
        let r = self.reference_spaces.find(&name);
        let s: &Space = Self::check_exactly_one(&r, "spaces", &name)?;

        Ok(SpaceId(s.name().clone()))
    }

    // Lookup a space within the reference spaces registered
    pub fn space_keys(&self) -> &Vec<String> {
        self.reference_spaces.keys()
    }

    // Lookup a space within the reference spaces registered
    pub fn space<S>(&self, name: S) -> Result<&Space, String>
    where
        S: Into<String>,
    {
        let name = name.into();
        if &name == space::Space::universe().name() {
            Ok(space::Space::universe())
        } else {
            let r = self.reference_spaces.find(&name);

            Self::check_exactly_one(&r, "spaces", &name)
        }
    }

    // Lookup a space within the reference spaces registered
    pub fn core_keys(&self) -> &Vec<String> {
        self.cores.keys()
    }

    // Lookup a dataset within the datasets registered
    pub fn core<S>(&self, name: S) -> Result<&Core, String>
    where
        S: Into<String>,
    {
        let name = name.into();
        let r = self.cores.find(&name);

        Self::check_exactly_one(&r, "cores", &name)
    }
}

impl ironsea_index::Record<String> for Space {
    fn key(&self) -> String {
        self.name().clone()
    }
}
