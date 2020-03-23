use serde::Deserialize;
use serde::Serialize;

use super::axis::Axis;
use super::coordinate::Coordinate;
use super::position::Position;
use super::MAX_K;

/// Kinds of space coordinate systems, or bases
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum CoordinateSystem {
    /// Absolute base, which allows to generate transformation between
    /// spaces by anchoring them relative to each other.
    Universe {
        /// A position which contains zeroes for all its coordinates,
        /// but has a coordinate per dimensions of the highest
        /// dimensions space referenced.
        origin: Position,
    },
    /// Base which needs only an affine transformation to map into the Universe.
    AffineSystem {
        /// Coordinates in Universe, expressed in f64, or decoded, and
        /// in the Universe number of dimensions.
        origin: Position,

        /// The definition of the coordinate system, through its axes.
        axes: Vec<Axis>,
    },
}

impl CoordinateSystem {
    /// Instantiate a new coordinate system.
    ///
    /// # Parameters
    ///
    ///  * `origin`:
    ///      The translation vector in Universe coordinates of this
    ///      base.
    ///
    ///  * `axes`:
    ///      The list of axes defining the coordinate system.
    pub fn new(origin: Vec<f64>, axes: Vec<Axis>) -> Self {
        CoordinateSystem::AffineSystem {
            origin: origin.into(),
            axes,
        }
    }

    /// The translation vector, in Universe coordinates.
    pub fn origin(&self) -> &Position {
        match self {
            CoordinateSystem::Universe { origin, .. } => origin,
            CoordinateSystem::AffineSystem { origin, .. } => origin,
        }
    }

    /// The axes definition of this base.
    pub fn axes(&self) -> &Vec<Axis> {
        match self {
            CoordinateSystem::Universe { .. } => {
                //FIXME: Generate a CoordinateSystem on the fly or store it as part of the Universe Space?
                unimplemented!()
            }
            CoordinateSystem::AffineSystem { axes, .. } => axes,
        }
    }

    /// The number of dimensions of positions within this base.
    pub fn dimensions(&self) -> usize {
        match self {
            CoordinateSystem::Universe { .. } => MAX_K,
            CoordinateSystem::AffineSystem { axes, .. } => axes.len(),
        }
    }

    /// The smallest bounding box containing the whole base, expressed
    /// in decoded Universe coordinates.
    ///
    // FIXME: Add the translation vector!
    pub fn bounding_box(&self) -> (Position, Position) {
        let mut low = Vec::with_capacity(self.dimensions());
        let mut high = Vec::with_capacity(self.dimensions());

        match self {
            CoordinateSystem::Universe { .. } => {
                for _ in 0..self.dimensions() {
                    low.push(std::f64::MIN);
                    high.push(std::f64::MAX);
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

    /// The volume of this space.
    ///
    // FIXME: This assumes orthogonal spaces!
    pub fn volume(&self) -> f64 {
        let (low, high) = self.bounding_box();
        let difference: Vec<_> = (high - low).into();

        let mut volume = 1.0;

        for l in difference {
            volume *= l;
        }

        volume
    }

    /// Rebase a position in this coordinate space.
    ///
    /// Each coordinate is encoded individually, and a new `Position`
    /// is generated.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed in decoded Universe coordinates.
    ///
    /// # Return value
    ///
    /// The encoded coordinates within this coordinate system.
    pub fn rebase(&self, position: &Position) -> Result<Position, String> {
        match self {
            CoordinateSystem::Universe { origin } => {
                // Ensure the coordinates are encoded into F64 variants of
                // coordinates by forcing an addition to the origin position
                // which is expressed as F64 variants. The addition will convert
                // to F64 automatically.
                Ok(origin + position)
            }
            CoordinateSystem::AffineSystem { origin, axes } => {
                let dimensions = axes.len();
                let translated = position - origin;
                let mut rebased = Vec::with_capacity(dimensions);

                for a in axes.iter().take(dimensions) {
                    let c = a.project_in(&translated)?;
                    rebased.push(c);
                }

                Ok(rebased.into())
            }
        }
    }

    /// Express the position in the Universe coordinate system.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed as an encoded coordinates in the coordinate system.
    ///
    /// # Return value
    ///
    /// The position expressed in Universe decoded coordinates.
    pub fn absolute_position(&self, position: &Position) -> Result<Position, String> {
        match self {
            CoordinateSystem::Universe { origin } => {
                // Ensure the coordinates are encoded into F64 variants of
                // coordinates by forcing an addition to the origin position
                // which is expressed as F64 variants. The addition will convert
                // to F64 automatically.
                Ok(origin + position)
            }
            CoordinateSystem::AffineSystem { axes, .. } => {
                // Start from the base origin.
                let mut rebased = self.origin().clone();

                // Convert to Universe coordinates
                for k in 0..axes.len() {
                    let c = axes[k].project_out(&position[k])?;
                    rebased += c;
                }

                Ok(rebased)
            }
        }
    }

    /// Encode a position expressed in the current coordinate system.
    ///
    /// Each coordinate is encoded individually, and a new `Position`
    /// is generated.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed in the current coordinate system.
    ///
    /// # Return value
    ///
    /// The encoded coordinates within this coordinate system.
    pub fn encode(&self, position: &[f64]) -> Result<Position, String> {
        let mut encoded = vec![];

        match self {
            CoordinateSystem::Universe { .. } => {
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

    /// Decode a position expressed in the current coordinate system as
    /// an encoded value.
    ///
    /// Each coordinate is decoded individually.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      expressed in the current coordinate system, as encoded
    ///      values.
    ///
    /// # Return value
    ///
    /// The decoded coordinates within this coordinate system.
    pub fn decode(&self, position: &Position) -> Result<Vec<f64>, String> {
        let mut decoded = vec![];

        match self {
            CoordinateSystem::Universe { .. } => {
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
