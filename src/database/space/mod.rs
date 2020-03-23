//! Reference space definitions.
//!
//! This include notions such as shapes, positions, axes, etc…

mod axis;
mod coordinate;
mod coordinate_system;
mod position;
mod shape;

#[cfg(test)]
mod tests;

use serde::Deserialize;
use serde::Serialize;

pub use axis::Axis;
pub use axis::Graduation;
pub use axis::NumberSet;
pub use coordinate::Coordinate;
pub use coordinate_system::CoordinateSystem;
pub use position::Position;
pub use shape::Shape;

// Maximum number of dimensions currently supported.
//
// **Note:** This will be deprecated as soon as support is implemented
//           in some dependencies. This is linked to limitations in
//           [ironsea_index_sfc_dbc].
//
// [ironsea_index_sfc_dbc]: https://github.com/epfl-dias/ironsea_index_sfc_dbc
const MAX_K: usize = 3;

lazy_static! {
    static ref UNIVERSE: Space = Space {
        name: "Universe".into(),
        system: CoordinateSystem::Universe {
            origin: [0f64; MAX_K].to_vec().into()
        },
    };
}

/// A reference space, defined by its name and coordinate system.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Space {
    name: String,
    system: CoordinateSystem,
}

impl Space {
    /// Instantiate a new space.
    ///
    /// # Parameters
    ///
    ///  * `name`:
    ///      Id of the reference space.
    ///
    ///  *  `system`:
    ///      Coordinate system defintion of the reference space
    pub fn new<S>(name: S, system: CoordinateSystem) -> Self
    where
        S: Into<String>,
    {
        Space {
            name: name.into(),
            system,
        }
    }

    /// Returns the Universe Space.
    ///
    /// This space contains all of the spaces, and allows us to connect
    /// them between each others.
    pub fn universe() -> &'static Self {
        &UNIVERSE
    }

    /// Transform a position from space `from` into a position in space `to`.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      Position to transform, expressed as encoded coordinates.
    ///
    ///  *  `from`:
    ///      Space in which `position` is defined.
    ///
    ///  *  `to`:
    ///      Target space in which `position` should be expressed.
    pub fn change_base(position: &Position, from: &Space, to: &Space) -> Result<Position, String> {
        to.rebase(&from.absolute_position(position)?)
    }

    /// Id of the reference space.
    pub fn name(&self) -> &String {
        &self.name
    }

    /// Origin of the space, expressed in Universe.
    pub fn origin(&self) -> &Position {
        self.system.origin()
    }

    /// Axes definition of the space.
    pub fn axes(&self) -> &Vec<Axis> {
        self.system.axes()
    }

    /// Returns the bounding box enclosing the whole space.
    pub fn bounding_box(&self) -> (Position, Position) {
        self.system.bounding_box()
    }

    /// Total volume of the reference space.
    pub fn volume(&self) -> f64 {
        self.system.volume()
    }

    // `position` is expressed in the Universe, this return encoded
    // coordinates in the current space.
    fn rebase(&self, position: &Position) -> Result<Position, String> {
        self.system.rebase(position)
    }

    // The position is expressed in encoded coordinates in the current space,
    // return an absolute position in Universe.
    fn absolute_position(&self, position: &Position) -> Result<Position, String> {
        self.system.absolute_position(position)
    }

    /// Decode coordinates expressed in the current space, to their
    /// values within the axes definitions.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed in encoded coordinates within the current space.
    ///
    /// # Return value
    ///
    /// The decoded position within the space.
    pub fn decode(&self, position: &Position) -> Result<Vec<f64>, String> {
        self.system.decode(position)
    }

    /// Encode a position expressed in the current space within the axes
    /// value ranges.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed in the current space.
    ///
    /// # Return value
    ///
    /// The encoded coordinates within the space.
    pub fn encode(&self, position: &[f64]) -> Result<Position, String> {
        self.system.encode(position)
    }
}
