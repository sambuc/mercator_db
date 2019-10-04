use super::coordinate::Coordinate;
use super::position::Position;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub enum NumberSet {
    N,
    Z,
    Q,
    R,
}

impl From<String> for NumberSet {
    fn from(set: String) -> Self {
        match set.as_str() {
            "N" => NumberSet::N,
            "Z" => NumberSet::Z,
            "Q" => NumberSet::Q,
            "R" => NumberSet::R,
            _ => panic!("Invalid set number: '{}', expected: N, Z, Q, R", set),
        }
    }
}

impl From<NumberSet> for String {
    fn from(set: NumberSet) -> String {
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

// TODO: In the future this might become an Enum with AffineAxis, ArbitraryAxis, etc...
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Axis {
    measurement_unit: String,
    graduation: Graduation,
    // Coordinates in Universe, expressed in f64, and in the Universe number of dimensions.
    pub unit_vector: Position,
}

impl Axis {
    pub fn new<S>(
        unit: S,
        unit_vector: Vec<f64>,
        set: NumberSet,
        minimum: f64,
        maximum: f64,
        steps: u64,
    ) -> Result<Self, String>
    where
        S: Into<String>,
    {
        // Convert to Position, and ensure it is a unit vector.
        let unit_vector = Position::from(unit_vector).unit();
        let graduation = Graduation::new(set, minimum, maximum, steps)?;

        Ok(Axis {
            measurement_unit: unit.into(),
            graduation,
            unit_vector,
        })
    }

    pub fn measurement_unit(&self) -> &String {
        &self.measurement_unit
    }

    pub fn unit_vector(&self) -> &Position {
        &self.unit_vector
    }

    pub fn graduation(&self) -> &Graduation {
        &self.graduation
    }

    // Project a point expressed from the origin of this axis on this axis.
    pub fn project_in(&self, position: &Position) -> Result<Coordinate, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        let d = position.dot_product(&self.unit_vector);

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            return Err(format!("Encode: position out of bounds: {} >= {}", d, max));
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            return Err(format!("Encode: position out of bounds: {} < {}", d, min));
        }

        self.encode(d)
    }

    // Convert a value on this axis to Universe coordinates, based from the origin of this axis.
    pub fn project_out(&self, coordinate: &Coordinate) -> Result<Position, String> {
        let d = self.decode(coordinate)?;
        Ok(self.unit_vector.clone() * d)
    }

    // Value is expressed on the current Axis, not in absolute coordinates!
    pub fn encode(&self, val: f64) -> Result<Coordinate, String> {
        let max = self.graduation.maximum;
        let min = self.graduation.minimum;

        let mut d = val;

        // Ensure it is within allowed range: Upper bound.
        if d > max {
            return Err(format!("Encode: position out of bounds: {} >= {}", d, max));
        }

        // Ensure it is within allowed range: Lower bound.
        if d < min {
            return Err(format!("Encode: position out of bounds: {} < {}", d, min));
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
