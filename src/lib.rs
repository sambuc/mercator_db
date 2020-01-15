#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate arrayref;

mod database;
pub mod storage;

pub use database::*;
