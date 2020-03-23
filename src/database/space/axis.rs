use serde::Deserialize;
use serde::Serialize;

use super::coordinate::Coordinate;
use super::position::Position;

/// Mathematical set numbers.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum NumberSet {
    /// [Natural numbers](https://en.wikipedia.org/wiki/Natural_number), here including **0**.
    N,
    /// [Integers](https://en.wikipedia.org/wiki/Integer).
    Z,
    /// [Rational](https://en.wikipedia.org/wiki/Rational_number) numbers.
    Q,
    /// [Real](https://en.wikipedia.org/wiki/Real_number) numbers.
    R,
}

impl From<&str> for NumberSet {
    fn from(set: &str) -> Self {
        match set {
            "N" => NumberSet::N,
            "Z" => NumberSet::Z,
            "Q" => NumberSet::Q,
            "R" => NumberSet::R,
            _ => panic!("Invalid set number: '{}', expected: N, Z, Q, R", set),
        }
    }
}

impl From<&NumberSet> for String {
    fn from(set: &NumberSet) -> String {
        let s = match set {
            NumberSet::N => "N",
            NumberSet::Z => "R",
            NumberSet::Q => "Q",
            NumberSet::R => "R",
        };

        s.to_string()
    }
}

/// Definition of a fixed-precision, finite length axis.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Graduation {
    /// Set of numbers allowed on the axis.
    pub set: NumberSet,
    /// Minimum value *inclusive*.
    pub minimum: f64,
    /// Maximum value *inclusive*.
    pub maximum: f64,
    /// Number of *ticks* or discrete values between `minimum` and
    /// `maximum`.
    pub steps: u64,
    /// Length between two distinct *ticks* on the axis.
    pub epsilon: f64,
}

impl Graduation {
    fn new(set: NumberSet, minimum: f64, maximum: f64, steps: u64) -> Result<Self, String> {
        Ok(Graduation {
            set,
            minimum,
            maximum,
            steps,
            epsilon: (maximum - minimum) / (steps as f64),
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[allow(non_camel_case_types)]
enum UnitSI {
    // Partial list, which is tailored to the use case needs. Prevents possible
    // confusions between Mm and mm, for example.
    m,
    dm,
    cm,
    mm,
    um,
    nm,
    pm,
}

impl UnitSI {
    pub fn factor(&self) -> f64 {
        match self {
            UnitSI::m => 1.0_E0,
            UnitSI::dm => 1.0_E-1,
            UnitSI::cm => 1.0_E-2,
            UnitSI::mm => 1.0_E-3,
            UnitSI::um => 1.0_E-6,
            UnitSI::nm => 1.0_E-9,
            UnitSI::pm => 1.0_E-12,
        }
    }

    pub fn to_str(&self) -> &str {
        match self {
            UnitSI::m => "m",
            UnitSI::dm => "dm",
            UnitSI::cm => "cm",
            UnitSI::mm => "mm",
            UnitSI::um => "um",
            UnitSI::nm => "nm",
            UnitSI::pm => "pm",
        }
    }
}

impl From<&str> for UnitSI {
    fn from(name: &str) -> Self {
        match name {
            "m" => UnitSI::m,
            "dm" => UnitSI::dm,
            "cm" => UnitSI::cm,
            "mm" => UnitSI::mm,
            "um" => UnitSI::um,
            "nm" => UnitSI::nm,
            "pm" => UnitSI::pm,
            _ => unimplemented!("Unknown unit '{}'", name),
        }
    }
}

/// Definition of an axis of a base.
///
/// This links together valid values on this axis, as well as the
/// direction in the Universe of the axis and the base length unit of
/// the `1.0` value.
// TODO: In the future this might become an Enum with AffineAxis, ArbitraryAxis, etc...
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Axis {
    measurement_unit: UnitSI,
    graduation: Graduation,
    // Coordinates in Universe, expressed in f64, and in the Universe
    // number of dimensions.
    unit_vector: Position,
}

impl Axis {
    /// Instanciate a new Axis definition.
    ///
    /// # Parameters
    ///
    ///  * `unit`:
    ///     SI Unit to use on this axis for the `1.0` value.
    ///     See [measurement_unit](#method.measurement_unit).
    ///
    ///  * `unit_vector`:
    ///     A vector providing the direction in the Universe space of
    ///     this axis.
    ///
    ///  * `set`:
    ///     The valid numbers on this axis.
    ///
    ///  * `minimum`:
    ///     The minimum value described by this axis *included*.
    ///
    ///  * `maximum`:
    ///     The maximum value described by this axis *included*.
    ///
    ///  * `steps`:
    ///     The number of steps, or discrete *ticks* on this axis.
    pub fn new(
        unit: &str,
        unit_vector: Vec<f64>,
        set: NumberSet,
        minimum: f64,
        maximum: f64,
        steps: u64,
    ) -> Result<Self, String> {
        // Convert to Position, and ensure it is a unit vector.
        let unit_vector = Position::from(unit_vector).unit();
        let graduation = Graduation::new(set, minimum, maximum, steps)?;

        Ok(Axis {
            measurement_unit: unit.into(),
            graduation,
            unit_vector,
        })
    }

    /// The unit, as in [SI unit] used on this axis, more specifically,
    /// a [metric prefix] of the **meter**.
    ///
    /// Currently the following values are supported:
    ///  * `m`
    ///  * `dm`
    ///  * `cm`
    ///  * `mm`
    ///  * `um`
    ///  * `nm`
    ///  * `pm`
    ///
    /// [SI unit]: https://en.wikipedia.org/wiki/International_System_of_Units
    /// [metric prefix]: https://en.wikipedia.org/wiki/Metric_prefix
    pub fn measurement_unit(&self) -> &str {
        self.measurement_unit.to_str()
    }

    /// The unit vector of the axis.
    ///
    /// This vector is expressed in the Universe coordinate system.
    pub fn unit_vector(&self) -> &Position {
        &self.unit_vector
    }

    /// The valid number range and properties on this axis.
    pub fn graduation(&self) -> &Graduation {
        &self.graduation
    }

    /// Project a position on this axis.
    ///
    /// The resulting coordinate is expressed as an encoded coordinate
    /// on this axis.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      The position to project on this axis. It must be defined in
    ///      Universe coordinates, but with any translations already
    ///      applied so that the origin of the vector is the origin of
    ///      this axis.
    pub fn project_in(&self, position: &Position) -> Result<Coordinate, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        let d = position.dot_product(&self.unit_vector);

        // Apply Unit scaling
        let mut d = d / self.measurement_unit.factor();

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            // FIXME: Should we generate an error instead?
            //return Err(format!(
            //    "project_in: position out of bounds: {} >= {}",
            //    d, max
            //));

            // FIXME: For now, just clip.
            d = max;
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            // FIXME: Should we generate an error instead?
            //return Err(format!(
            //    "project_in: position out of bounds: {} < {}",
            //    d, min
            //));

            // FIXME: For now, just clip.
            d = min;
        }

        self.encode(d)
    }

    /// Convert an encoded coordinate expressed on this axis into a
    /// position.
    ///
    /// The resulting position is expressed in the Universe reference
    /// space, but from the origin of this axis. Any required
    /// translation must be applied to this resulting position to obtain
    /// an absolute value in the Universe space.
    ///
    /// # Parameters
    ///
    ///  * `coordinate`:
    ///      The coordinate to project out of this axis. It must be
    ///      defined as an encoded coordinate on this axis.
    pub fn project_out(&self, coordinate: &Coordinate) -> Result<Position, String> {
        let d = self.decode(coordinate)?;

        // Apply Unit scaling
        let d = d * self.measurement_unit.factor();

        Ok(&self.unit_vector * d)
    }

    /// Encode a coordinate expressed on this axis.
    ///
    /// # Parameters
    ///
    ///  * `val`:
    ///      The coordinate to encode. It must be defined as a
    ///      coordinate on this axis.
    pub fn encode(&self, val: f64) -> Result<Coordinate, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        let mut d = val;

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            return Err(format!("encode: position out of bounds: {} >= {}", d, max));
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            return Err(format!("encode: position out of bounds: {} < {}", d, min));
        }

        // Shift range to zero.
        d -= min;

        // Scale to range.
        let v = (d / self.graduation.epsilon) as u64;

        // Convert to appropriate type.
        Ok(v.into())
    }

    /// Decode a coordinate expressed on this axis.
    ///
    /// # Parameters
    ///
    ///  * `val`:
    ///      The coordinate to decode. It must be defined as an encoded
    ///      coordinate on this axis.
    pub fn decode(&self, val: &Coordinate) -> Result<f64, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        // Convert to appropriate type.
        let mut d = val.f64();

        // Scale range back.
        d *= self.graduation.epsilon;

        // Shift range back to origin.
        d += self.graduation.minimum;

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            return Err(format!("Decode: position out of bounds: {} >= {}", d, max));
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            return Err(format!("Decode: position out of bounds: {} < {}", d, min));
        }

        Ok(d)
    }
}
