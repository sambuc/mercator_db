use std::cmp::Ordering;
use std::collections::HashSet;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::iter::FromIterator;
use std::ops::Add;
use std::ops::AddAssign;
use std::ops::Index;
use std::ops::IndexMut;
use std::ops::Mul;
use std::ops::MulAssign;
use std::ops::Sub;
use std::ops::SubAssign;

use serde::Deserialize;
use serde::Serialize;

use super::coordinate::Coordinate;

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, Serialize)]
pub enum Position {
    Position1(Coordinate),
    Position2([Coordinate; 2]),
    Position3([Coordinate; 3]),
    Position4([Coordinate; 4]),
    Position5([Coordinate; 5]),
    Position6([Coordinate; 6]),
    Position7([Coordinate; 7]),
    Position8([Coordinate; 8]),
    PositionN(Vec<Coordinate>),
}

impl Position {
    pub fn new(coordinates: Vec<Coordinate>) -> Self {
        coordinates.into()
    }

    pub fn dimensions(&self) -> usize {
        match self {
            Position::Position1(_) => 1,
            Position::Position2(_) => 2,
            Position::Position3(_) => 3,
            Position::Position4(_) => 4,
            Position::Position5(_) => 5,
            Position::Position6(_) => 6,
            Position::Position7(_) => 7,
            Position::Position8(_) => 8,
            Position::PositionN(coordinates) => coordinates.len(),
        }
    }

    // Returns ||self||
    pub fn norm(&self) -> f64 {
        if let Position::Position1(coordinates) = self {
            // the square root of a single number to the square is its positive value, so ensure it is.
            coordinates.f64().abs()
        } else {
            let point: Vec<&Coordinate> = self.into();
            let mut squared = 0f64;

            for c in point {
                let t: f64 = c.into();
                squared += t * t;
            }

            squared.sqrt()
        }
    }

    // Unit / Normalized vector from self.
    pub fn unit(&self) -> Self {
        self * (1f64 / self.norm())
    }

    // This multiplies self^T with other, producing a scalar value
    pub fn dot_product(&self, other: &Self) -> f64 {
        assert_eq!(self.dimensions(), other.dimensions());

        let mut product = 0f64;

        for k in 0..self.dimensions() {
            product += (self[k] * other[k]).f64();
        }

        product
    }

    pub fn reduce_precision(&self, scale: u32) -> Self {
        let mut position = Vec::with_capacity(self.dimensions());

        for i in 0..self.dimensions() {
            position.push((self[i].u64() >> scale).into())
        }

        Position::new(position)
    }
}

impl Display for Position {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        let v: Vec<&Coordinate> = self.into();
        write!(f, "{:?}", v)
    }
}

impl PartialOrd for Position {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // Let's restrict for now to same-length vectors.
        if self.dimensions() != other.dimensions() {
            return None;
        }

        let mut ordering = HashSet::with_capacity(self.dimensions());
        for k in 0..self.dimensions() {
            ordering.insert(self[k].partial_cmp(&other[k]));
        }

        if ordering.contains(&None) {
            return None;
        }

        let ordering = ordering.drain().filter_map(|v| v).collect::<Vec<_>>();
        match ordering.len() {
            3 => None,
            2 => {
                // The two values are, by construction different, which means we
                // have the following possibilities as there are only GREATER,
                // EQUAL and LESS in the enum:
                //  - LESS, GREATER
                //  - LESS, EQUAL
                //  - GREATER, EQUAL
                // If one of the values is EQUAL, then the ordering will be the
                // other value.
                if ordering[0] == Ordering::Equal {
                    Some(ordering[1])
                } else if ordering[1] == Ordering::Equal {
                    Some(ordering[0])
                } else {
                    None
                }
            }
            1 => Some(ordering[0]),
            // We can not have more than 3 elements, and if we have 0, it means
            // we had only None in the list
            _ => None,
        }
    }
}

impl Index<usize> for Position {
    type Output = Coordinate;

    fn index(&self, k: usize) -> &Self::Output {
        match self {
            Position::Position1(coordinate) => coordinate,
            Position::Position2(coordinates) => &coordinates[k],
            Position::Position3(coordinates) => &coordinates[k],
            Position::Position4(coordinates) => &coordinates[k],
            Position::Position5(coordinates) => &coordinates[k],
            Position::Position6(coordinates) => &coordinates[k],
            Position::Position7(coordinates) => &coordinates[k],
            Position::Position8(coordinates) => &coordinates[k],
            Position::PositionN(coordinates) => &coordinates[k],
        }
    }
}

impl IndexMut<usize> for Position {
    fn index_mut(&mut self, k: usize) -> &mut Self::Output {
        match self {
            Position::Position1(coordinate) => coordinate,
            Position::Position2(coordinates) => &mut coordinates[k],
            Position::Position3(coordinates) => &mut coordinates[k],
            Position::Position4(coordinates) => &mut coordinates[k],
            Position::Position5(coordinates) => &mut coordinates[k],
            Position::Position6(coordinates) => &mut coordinates[k],
            Position::Position7(coordinates) => &mut coordinates[k],
            Position::Position8(coordinates) => &mut coordinates[k],
            Position::PositionN(coordinates) => &mut coordinates[k],
        }
    }
}

impl Add for Position {
    type Output = Position;

    fn add(mut self, rhs: Self) -> Self::Output {
        self += rhs;
        self
    }
}

impl Add for &Position {
    type Output = Position;

    fn add(self, rhs: Self) -> Self::Output {
        let dimensions = self.dimensions();
        assert_eq!(dimensions, rhs.dimensions());
        let mut v = Vec::with_capacity(dimensions);

        for k in 0..dimensions {
            v.push(self[k] + rhs[k]);
        }

        v.into()
    }
}

impl AddAssign for Position {
    fn add_assign(&mut self, rhs: Self) {
        let dimensions = self.dimensions();
        assert_eq!(dimensions, rhs.dimensions());

        for k in 0..dimensions {
            self[k] = self[k] + rhs[k];
        }
    }
}

impl Sub for Position {
    type Output = Position;

    fn sub(mut self, rhs: Self) -> Self::Output {
        self -= rhs;
        self
    }
}

impl Sub for &Position {
    type Output = Position;

    fn sub(self, rhs: Self) -> Self::Output {
        let dimensions = self.dimensions();
        assert_eq!(dimensions, rhs.dimensions());
        let mut v = Vec::with_capacity(dimensions);

        for k in 0..dimensions {
            v.push(self[k] - rhs[k]);
        }

        v.into()
    }
}

impl SubAssign for Position {
    fn sub_assign(&mut self, rhs: Self) {
        let dimensions = self.dimensions();
        assert_eq!(dimensions, rhs.dimensions());

        for k in 0..dimensions {
            self[k] = self[k] - rhs[k];
        }
    }
}

// Scalar product
impl Mul<f64> for Position {
    type Output = Position;

    fn mul(mut self, rhs: f64) -> Self::Output {
        self *= rhs;
        self
    }
}

impl Mul<f64> for &Position {
    type Output = Position;

    fn mul(self, rhs: f64) -> Self::Output {
        let dimensions = self.dimensions();
        let mut v = Vec::with_capacity(dimensions);

        for k in 0..dimensions {
            v.push(self[k] * rhs);
        }

        v.into()
    }
}

// Scalar product
impl MulAssign<f64> for Position {
    fn mul_assign(&mut self, rhs: f64) {
        for k in 0..self.dimensions() {
            self[k] = self[k] * rhs;
        }
    }
}

// Outer product
impl Mul for Position {
    type Output = Vec<Position>;

    fn mul(self, rhs: Self) -> Self::Output {
        let mut m = Vec::with_capacity(rhs.dimensions());

        for i in 0..rhs.dimensions() {
            let mut u = Vec::with_capacity(self.dimensions());

            for k in 0..self.dimensions() {
                u[k] = self[k] * rhs[i];
            }
            m[i] = u.into();
        }

        m
    }
}

// FIXME: Which is faster, the code below or the automatically generated
//        implementation?
/*
impl PartialEq for Position {
    fn eq(&self, other: &Self) -> bool {
        for i in 0..self.dimensions() {
            if self[i] != other[i] {
                return false;
            }
        }
        true
    }
}
*/

impl<'s> From<&'s Position> for Vec<&'s Coordinate> {
    fn from(position: &'s Position) -> Self {
        match position {
            Position::Position1(coordinate) => vec![coordinate],
            Position::Position2(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position3(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position4(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position5(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position6(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position7(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::Position8(coordinates) => coordinates.iter().map(|c| c).collect(),
            Position::PositionN(coordinates) => coordinates.iter().map(|c| c).collect(),
        }
    }
}

impl From<Vec<Coordinate>> for Position {
    fn from(coordinates: Vec<Coordinate>) -> Self {
        match coordinates.len() {
            1 => Position::Position1(coordinates[0]),
            2 => Position::Position2(*array_ref!(coordinates, 0, 2)),
            3 => Position::Position3(*array_ref!(coordinates, 0, 3)),
            4 => Position::Position4(*array_ref!(coordinates, 0, 4)),
            5 => Position::Position5(*array_ref!(coordinates, 0, 5)),
            6 => Position::Position6(*array_ref!(coordinates, 0, 6)),
            7 => Position::Position7(*array_ref!(coordinates, 0, 7)),
            8 => Position::Position8(*array_ref!(coordinates, 0, 8)),
            _ => Position::PositionN(coordinates),
        }
    }
}

impl From<Vec<f64>> for Position {
    fn from(coordinates: Vec<f64>) -> Self {
        coordinates
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<Coordinate>>()
            .into()
    }
}

impl From<&Vec<f64>> for Position {
    fn from(coordinates: &Vec<f64>) -> Self {
        coordinates
            .iter()
            .map(|c| (*c).into())
            .collect::<Vec<Coordinate>>()
            .into()
    }
}

impl From<Vec<u64>> for Position {
    fn from(coordinates: Vec<u64>) -> Self {
        coordinates
            .into_iter()
            .map(|c| c.into())
            .collect::<Vec<Coordinate>>()
            .into()
    }
}

impl From<Position> for Vec<f64> {
    fn from(position: Position) -> Self {
        (&position).into()
    }
}

impl From<&Position> for Vec<f64> {
    fn from(position: &Position) -> Self {
        let point: Vec<&Coordinate> = position.into();

        point.into_iter().map(|c| c.into()).collect()
    }
}

impl FromIterator<f64> for Position {
    fn from_iter<I: IntoIterator<Item = f64>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}

impl FromIterator<Coordinate> for Position {
    fn from_iter<I: IntoIterator<Item = Coordinate>>(iter: I) -> Self {
        iter.into_iter().collect::<Vec<_>>().into()
    }
}
