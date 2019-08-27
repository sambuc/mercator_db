# Mercator DB

Database model for the Mercator spatial index.

## Mercator: Spatial Index

**Mercator** is a spatial *volumetric* index for the [Human Brain Project](http://www.humanbrainproject.eu). It is a component of the [Knowledge Graph](http://www.humanbrainproject.eu/en/explore-the-brain/search/) service, which  provides the spatial anchoring for the metadata registered as well as processes the volumetric queries.

It is build on top of the Iron Sea database toolkit.

## Iron Sea: Database Toolkit

**Iron Sea** provides a set of database engine bricks, which can be combined and applied on arbitrary data structures.

Unlike a traditional database, it does not assume a specific physical structure for the tables nor the records, but relies on the developper to provide a set of extractor functions which are used by the specific indices provided.

This enables the index implementations to be agnostic from the underlying data structure, and re-used.

## Requirements

### Software

 * Rust: https://www.rust-lang.org

## Quick start

## Building from sources

To build this project, you will need to run the following:

```sh
cargo build --release
```

### Installation

To install the software on the system you can use:

```sh
cargo install --release
```

### Usage

The binary `db-test` provided is used only as an integration test at this point. It will convert a json input to a binary representation, before building an index over it. Once this is achieved, it will run a couple of hard-coded queries over the index.

```sh
cargo run --release
```

## Documentation

For more information, please refer to the [documentation](https://epfl-dias.github.io/PROJECT_NAME/).

If you want to build the documentation and access it locally, you can use:

```sh
cargo doc --open
```

## Acknowledgements

This open source software code was developed in part or in whole in the
Human Brain Project, funded from the European Union’s Horizon 2020
Framework Programme for Research and Innovation under the Specific Grant
Agreement No. 785907 (Human Brain Project SGA2).
