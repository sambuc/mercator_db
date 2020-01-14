#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate arrayref;

#[macro_use]
extern crate serde_derive;

mod database;
pub mod storage;

pub use database::*;
