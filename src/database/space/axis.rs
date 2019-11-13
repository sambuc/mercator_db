use super::coordinate::Coordinate;
use super::position::Position;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum NumberSet {
    N,
    Z,
    Q,
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

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Graduation {
    pub set: NumberSet,
    pub minimum: f64,
    pub maximum: f64,
    pub steps: u64,
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
pub enum UnitSI {
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

// TODO: In the future this might become an Enum with AffineAxis, ArbitraryAxis, etc...
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Axis {
    measurement_unit: UnitSI,
    graduation: Graduation,
    // Coordinates in Universe, expressed in f64, and in the Universe number of dimensions.
    pub unit_vector: Position,
}

impl Axis {
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

    pub fn measurement_unit(&self) -> &str {
        self.measurement_unit.to_str()
    }

    pub fn unit_vector(&self) -> &Position {
        &self.unit_vector
    }

    pub fn graduation(&self) -> &Graduation {
        &self.graduation
    }

    // Project a point expressed in Universe coordinates from the origin of this
    // axis on this axis.
    pub fn project_in(&self, position: &Position) -> Result<Coordinate, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        let d = position.dot_product(&self.unit_vector);

        // Apply Unit scaling
        let d = d / self.measurement_unit.factor();

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            return Err(format!(
                "project_in: position out of bounds: {} >= {}",
                d, max
            ));
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            return Err(format!(
                "project_in: position out of bounds: {} < {}",
                d, min
            ));
        }

        self.encode(d)
    }

    // Convert a value on this axis to Universe coordinates, based from the origin of this axis.
    pub fn project_out(&self, coordinate: &Coordinate) -> Result<Position, String> {
        let d = self.decode(coordinate)?;

        // Apply Unit scaling
        let d = d * self.measurement_unit.factor();

        Ok(&self.unit_vector * d)
    }

    // Value is expressed on the current Axis, not in absolute coordinates!
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

    // Value is expressed on the current Axis, not in absolute coordinates!
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
