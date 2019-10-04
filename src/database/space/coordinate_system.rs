use super::axis::Axis;
use super::coordinate::Coordinate;
use super::position::Position;
use super::MAX_K;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CoordinateSystem {
    Universe,
    // Coordinates in Universe, expressed in f64, and in the Universe number of dimensions.
    AffineSystem { origin: Position, axes: Vec<Axis> },
}

impl CoordinateSystem {
    pub fn new(origin: Vec<f64>, axes: Vec<Axis>) -> Self {
        CoordinateSystem::AffineSystem {
            origin: origin.into(),
            axes,
        }
    }

    pub fn origin(&self) -> Position {
        match self {
            CoordinateSystem::Universe => {
                let origin = [0f64; MAX_K].to_vec();
                origin.into()
            }
            CoordinateSystem::AffineSystem { origin, .. } => origin.clone(),
        }
    }

    pub fn axes(&self) -> Vec<Axis> {
        match self {
            CoordinateSystem::Universe => {
                //FIXME: Generate a CoordinateSystem on the fly or store it as part of the Universe Space?
                unimplemented!()
            }
            CoordinateSystem::AffineSystem { axes, .. } => axes.clone(),
        }
    }

    pub fn dimensions(&self) -> usize {
        match self {
            CoordinateSystem::Universe => MAX_K,
            CoordinateSystem::AffineSystem { axes, .. } => axes.len(),
        }
    }

    pub fn bounding_box(&self) -> (Position, Position) {
        let mut low = Vec::with_capacity(self.dimensions());
        let mut high = Vec::with_capacity(self.dimensions());

        match self {
            CoordinateSystem::Universe => {
                for _ in 0..self.dimensions() {
                    low.push(std::f64::MAX);
                    high.push(std::f64::MIN);
                }
            }
            CoordinateSystem::AffineSystem { axes, .. } => {
                for a in axes {
                    low.push(a.graduation().minimum);
                    high.push(a.graduation().maximum);
                }
            }
        }

        (low.into(), high.into())
    }

    pub fn volume(&self) -> f64 {
        let (low, high) = self.bounding_box();
        let difference: Vec<_> = (high - low).into();

        let mut volume = 1.0;

        for l in difference {
            volume *= l;
        }

        volume
    }

    // The position is expressed in coordinates in the universe,
    // return a position in the current coordinate system.
    pub fn rebase(&self, position: &Position) -> Result<Position, String> {
        match self {
            CoordinateSystem::Universe => {
                // Ensure the coordinates are encoded into F64 variants of
                // coordinates by forcing an addition to the origin position
                // which is expressed as F64 variants. The addition will convert
                // to F64 automatically.
                Ok(self.origin().clone() + position.clone())
            }
            CoordinateSystem::AffineSystem { origin, axes } => {
                let dimensions = axes.len();
                let translated = position.clone() - origin.clone();
                let mut rebased = Vec::with_capacity(dimensions);

                for a in axes.iter().take(dimensions) {
                    let c = a.project_in(&translated)?;
                    rebased.push(c);
                }

                Ok(rebased.into())
            }
        }
    }

    // The position is expressed in coordinates in the current coordinate system,
    // return a position in Universe coordinates.
    pub fn absolute_position(&self, position: &Position) -> Result<Position, String> {
        match self {
            CoordinateSystem::Universe => {
                // Ensure the coordinates are encoded into F64 variants of
                // coordinates by forcing an addition to the origin position
                // which is expressed as F64 variants. The addition will convert
                // to F64 automatically.
                Ok(self.origin().clone() + position.clone())
            }
            CoordinateSystem::AffineSystem { axes, .. } => {
                // Start from the base origin.
                let mut rebased = self.origin();

                // Convert to Universe coordinates
                for k in 0..axes.len() {
                    let c = axes[k].project_out(&position[k])?;
                    rebased += c;
                }

                Ok(rebased)
            }
        }
    }

    // The position is expressed in the current system
    // Encode each coordinate separately and return an encoded Position
    pub fn encode(&self, position: &[f64]) -> Result<Position, String> {
        let mut encoded = vec![];

        match self {
            CoordinateSystem::Universe => {
                assert_eq!(position.len(), MAX_K);
                for c in position {
                    encoded.push(Coordinate::CoordinateF64(*c));
                }
            }
            CoordinateSystem::AffineSystem { axes, .. } => {
                assert_eq!(position.len(), axes.len());
                for k in 0..axes.len() {
                    encoded.push(axes[k].encode(position[k])?);
                }
            }
        };

        Ok(encoded.into())
    }

    // The position is expressed in the current system as an encoded value,
    // return a position in the current system as f64 values.
    pub fn decode(&self, position: &Position) -> Result<Vec<f64>, String> {
        let mut decoded = vec![];

        match self {
            CoordinateSystem::Universe => {
                assert_eq!(position.dimensions(), MAX_K);
                for c in 0..position.dimensions() {
                    decoded.push(position[c].into());
                }
            }
            CoordinateSystem::AffineSystem { axes, .. } => {
                assert_eq!(position.dimensions(), axes.len());
                for k in 0..axes.len() {
                    decoded.push(axes[k].decode(&position[k])?);
                }
            }
        };

        Ok(decoded)
    }
}
