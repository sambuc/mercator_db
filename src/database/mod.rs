mod db_core;
pub mod space;
mod space_db;
pub(crate) mod space_index;

use std::collections::HashMap;

use ironsea_index::Indexed;

use super::storage;
pub use db_core::Core;
pub use db_core::CoreQueryParameters;
pub use db_core::Properties;
use space::Position;
use space::Space;

/// TODO doc
pub type IterPositions<'i> = Box<dyn Iterator<Item = Position> + 'i>;
/// TODO doc
pub type IterObjects<'i> = Box<dyn Iterator<Item = (Position, &'i Properties)> + 'i>;
/// TODO doc
pub type IterObjectsBySpaces<'i> = Vec<(&'i String, IterObjects<'i>)>;

/// Selected tuples matching a query.
///
/// This is either:
///  * `Err` with a reason stored as a `String`
///  * `Ok`, with a vector of tuples defined as:
///        `(Space Name, [(Position, Properties)])`
pub type ResultSet<'r> = Result<IterObjectsBySpaces<'r>, String>;

type ReferenceSpaceIndex = ironsea_index_hashmap::Index<Space, String>;
type CoreIndex = ironsea_index_hashmap::Index<Core, String>;

/// Collection of datasets and their reference spaces.
pub struct DataBase {
    reference_spaces: ReferenceSpaceIndex,
    cores: CoreIndex,
}

impl DataBase {
    /// Instantiate a `DataBase` struct.
    ///
    /// # Parameters
    ///
    ///  * `spaces`:
    ///      List of reference spaces.
    ///
    ///  * `cores`:
    ///      List of datasets (cores) which will be queried through this
    ///      `DataBase` struct.
    // TODO: Replace vectors with iterators?
    pub fn new(spaces: Vec<Space>, cores: Vec<Core>) -> Self {
        DataBase {
            reference_spaces: ReferenceSpaceIndex::new(spaces.into_iter()),
            cores: CoreIndex::new(cores.into_iter()),
        }
    }

    /// Load a list of indices.
    ///
    /// # Parameters
    ///
    ///  * `indices`:
    ///      The list of index file names to load.
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

    fn load_core(name: &str) -> Result<(Vec<Space>, Core), String> {
        match storage::bincode::load(name) {
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

    /// Returns an ordered list of the reference space names registered.
    pub fn space_keys(&self) -> &Vec<String> {
        self.reference_spaces.keys()
    }

    /// Lookup a space within the reference spaces registered.
    ///
    /// # Parameters
    ///
    ///  * `name`:
    ///      The name of the reference space to search for.
    pub fn space(&self, name: &str) -> Result<&Space, String> {
        if name == space::Space::universe().name() {
            Ok(space::Space::universe())
        } else {
            let r = self
                .reference_spaces
                .find(&name.to_string())
                .collect::<Vec<_>>();

            Self::check_exactly_one(&r, "spaces", name)
        }
    }

    /// Returns an ordered list of dataset (Core) names registered.
    pub fn core_keys(&self) -> &Vec<String> {
        self.cores.keys()
    }

    /// Lookup a dataset within the datasets registered.
    ///
    /// # Parameters
    ///
    ///  * `name`:
    ///      The name of the dataset (core) to search for.
    pub fn core(&self, name: &str) -> Result<&Core, String> {
        let r = self.cores.find(&name.to_string()).collect::<Vec<_>>();

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
