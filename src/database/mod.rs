mod db_core;
pub mod space;
mod space_db;
mod space_index;

use std::collections::HashMap;
use std::fs::File;

use ironsea_index::Indexed;
use memmap::Mmap;

pub use db_core::Core;
pub use db_core::CoreQueryParameters;
pub use db_core::Properties;
use space::Position;
use space::Space;
pub use space_index::SpaceFields;
pub use space_index::SpaceSetObject;

// (Space Name, Position, Fields)
pub type ResultSet<'r> = Result<Vec<(&'r String, Vec<(Position, &'r Properties)>)>, String>;
pub type ReferenceSpaceIndex = ironsea_index_hashmap::Index<Space, String>;
type CoreIndex = ironsea_index_hashmap::Index<Core, String>;

#[derive(Clone, Debug, Eq, Hash, PartialEq, Serialize)]
pub struct SpaceObject {
    pub space_id: String,
    pub position: Position,
    pub value: Properties,
}

pub struct DataBase {
    reference_spaces: ReferenceSpaceIndex,
    cores: CoreIndex,
}

impl DataBase {
    pub fn new(spaces: Vec<Space>, cores: Vec<Core>) -> Self {
        DataBase {
            reference_spaces: ReferenceSpaceIndex::new(spaces.into_iter()),
            cores: CoreIndex::new(cores.into_iter()),
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

    fn load_core(name: &str) -> Result<(Vec<Space>, Core), String> {
        let mmap = DataBase::mmap_file(&name)?;

        match bincode::deserialize(&mmap[..]) {
            Err(e) => Err(format!("Index deserialization error: {:?}", e)),
            Ok(index) => Ok(index),
        }
    }

    fn check_exactly_one<'t, T>(list: &[&'t T], name: &str, value: &str) -> Result<&'t T, String> {
        if list.len() > 1 {
            Err(format!(
                "Multiple {} registered under `{}`: {}",
                name,
                value,
                list.len()
            ))
        } else if list.is_empty() {
            Err(format!(
                "No {} registered under `{}`: {}",
                name,
                value,
                list.len()
            ))
        } else {
            Ok(&list[0])
        }
    }

    // Lookup a space within the reference spaces registered
    pub fn space_keys(&self) -> &Vec<String> {
        self.reference_spaces.keys()
    }

    // Lookup a space within the reference spaces registered
    pub fn space(&self, name: &str) -> Result<&Space, String> {
        if name == space::Space::universe().name() {
            Ok(space::Space::universe())
        } else {
            let r = self.reference_spaces.find(&name.to_string());

            Self::check_exactly_one(&r, "spaces", name)
        }
    }

    // Lookup a space within the reference spaces registered
    pub fn core_keys(&self) -> &Vec<String> {
        self.cores.keys()
    }

    // Lookup a dataset within the datasets registered
    pub fn core(&self, name: &str) -> Result<&Core, String> {
        let r = self.cores.find(&name.to_string());

        Self::check_exactly_one(&r, "cores", name)
    }
}

impl ironsea_index::Record<String> for Space {
    fn key(&self) -> String {
        self.name().clone()
    }
}

impl ironsea_index::Record<String> for Core {
    fn key(&self) -> String {
        self.name().clone()
    }
}
