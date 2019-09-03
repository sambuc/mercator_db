#[macro_use]
extern crate measure_time;

#[macro_use]
extern crate serde_derive;

use mercator_db::json::model;
use mercator_db::json::storage;
use mercator_db::space::Shape;
use mercator_db::DataBase;

fn main() {
    // If RUST_LOG is unset, set it to INFO, otherwise keep it as-is.
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "info");
    }
    pretty_env_logger::init();

    // Convert to binary the JSON data:
    if true {
        info_time!("Converting to binary JSON data");
        storage::convert("10k");
    }

    // Build a Database Index:
    if true {
        info_time!("Building database index");
        storage::build("10k");
    }

    // Load a Database:
    let db;
    {
        info_time!("Loading database index");
        db = DataBase::load("10k").unwrap();
    }

    if true {
        let core = db.core("10k").unwrap();
        let space = db.space("std").unwrap();
        let id = "oid0.5793259558369925";

        let r = core.get_by_id(&db, id, None, std::f64::MAX).unwrap();
        println!("get_by_id {}: {}", id, r.len());
        println!("{}: {:?}\n", id, r[0]);

        let r = core.get_by_id(&db, id, None, 0.0).unwrap();
        println!("get_by_id {}: {}", id, r.len());
        println!("{}: {:?}\n", id, r[0]);

        let r = core.get_by_label(&db, id, None, std::f64::MAX).unwrap();
        println!("get_by_label {}: {}", id, r.len());
        if !r.is_empty() {
            println!("{}: {:?}\n", id, r[0]);
        }

        let lower = space.encode(&[0.2, 0.2, 0.2]).unwrap();
        let higher = space.encode(&[0.8, 0.8, 0.8]).unwrap();

        let shape = Shape::BoundingBox(lower, higher);

        let r = core.get_by_shape(&db, &shape, "std", None, 0.0).unwrap();
        println!("get_by_shape {:?}: {}", shape, r.len());
        println!("{:?}: {:?}\n", shape, r[0]);

        let a = r.iter().filter(|o| o.value.id() == id).collect::<Vec<_>>();
        println!("get_by_shape A {:?} filtered on {}: {}", shape, id, a.len());
        if !a.is_empty() {
            println!("{:?}\n", a[0]);
        }

        let a = r.iter().filter(|o| o.value.id() != id).collect::<Vec<_>>();
        println!(
            "get_by_shape !A {:?} filtered on {}: {}",
            shape,
            id,
            a.len()
        );
        if !a.is_empty() {
            println!("{:?}\n", a[0]);
        }

        println!(
            "\nSPACE OBJECT:\n\n{}",
            serde_json::to_string_pretty(space).unwrap()
        );
        println!(
            "\nSPATIAL OBJECT:\n\n{}",
            serde_json::to_string_pretty(a[0]).unwrap()
        );
    }
}
