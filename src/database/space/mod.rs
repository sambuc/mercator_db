mod axis;
mod coordinate;
mod coordinate_system;
mod position;
mod shape;

#[cfg(test)]
mod tests;

pub use axis::Axis;
pub use axis::Graduation;
pub use axis::NumberSet;
pub use coordinate::Coordinate;
pub use coordinate_system::CoordinateSystem;
pub use position::Position;
pub use shape::Shape;

pub const MAX_K: usize = 3;

lazy_static! {
    static ref UNIVERSE: Space = Space {
        name: "Universe".into(),
        system: CoordinateSystem::Universe,
    };
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Space {
    name: String,
    system: CoordinateSystem,
}

impl Space {
    pub fn new<S>(name: S, system: CoordinateSystem) -> Self
    where
        S: Into<String>,
    {
        Space {
            name: name.into(),
            system,
        }
    }

    pub fn universe() -> &'static Self {
        &UNIVERSE
    }

    pub fn change_base(position: &Position, from: &Space, to: &Space) -> Result<Position, String> {
        to.rebase(&from.absolute_position(position)?)
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn origin(&self) -> Position {
        self.system.origin()
    }

    pub fn axes(&self) -> Vec<Axis> {
        self.system.axes()
    }

    pub fn bounding_box(&self) -> (Position, Position) {
        self.system.bounding_box()
    }

    pub fn volume(&self) -> f64 {
        self.system.volume()
    }

    // The position is expressed in coordinates in the universe,
    // return a position in the current space.
    pub fn rebase(&self, position: &Position) -> Result<Position, String> {
        self.system.rebase(position)
    }

    // The position is expressed in coordinates in the current space,
    // return an absolute position in Universe.
    pub fn absolute_position(&self, position: &Position) -> Result<Position, String> {
        self.system.absolute_position(position)
    }

    // The position is expressed in the current space as an encoded value,
    // return a position in the current system as f64 values
    pub fn decode(&self, position: &Position) -> Result<Vec<f64>, String> {
        self.system.decode(position)
    }

    // The position is expressed in the current space,
    // return a position expressed in the current space as an encoded value.
    pub fn encode(&self, position: &[f64]) -> Result<Position, String> {
        self.system.encode(position)
    }
}
