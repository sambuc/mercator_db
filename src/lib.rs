#![deny(missing_docs)]

//! # Mercator DB
//!
//! Database model for the Mercator spatial index.
//!
//! ## Mercator: Spatial Index
//!
//! **Mercator** is a spatial *volumetric* index for the
//! [Human Brain Project]. It is a component of the [Knowledge Graph]
//! service, which  provides the spatial anchoring for the metadata
//! registered as well as processes the volumetric queries.
//!
//! It is build on top of the Iron Sea database toolkit.
//!
//! ## Iron Sea: Database Toolkit
//! **Iron Sea** provides a set of database engine bricks, which can be
//! combined and applied on arbitrary data structures.
//!
//! Unlike a traditional database, it does not assume a specific
//! physical structure for the tables nor the records, but relies on the
//! developer to provide a set of extractor functions which are used by
//! the specific indices provided.
//!
//! This enables the index implementations to be agnostic from the
//! underlying data structure, and re-used.
//!
//! [Human Brain Project]: http://www.humanbrainproject.eu
//! [Knowledge Graph]: http://www.humanbrainproject.eu/en/explore-the-brain/search/

#[macro_use]
extern crate lazy_static;

#[macro_use]
extern crate log;

#[macro_use]
extern crate arrayref;

mod database;
pub mod storage;

pub use database::*;
