use std::cmp::Ordering;
use std::fmt;
use std::fmt::Display;
use std::fmt::Formatter;
use std::hash::Hash;
use std::hash::Hasher;
use std::ops::Add;
use std::ops::Mul;
use std::ops::Sub;

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
pub enum Coordinate {
    CoordinateU8(u8),
    CoordinateU16(u16),
    CoordinateU32(u32),
    CoordinateU64(u64),
    // We currently assume that 2^64 is enough to store encoded position values per axis.
    //CoordinateU128(u128),
    CoordinateF64(f64),
}

impl Coordinate {
    pub fn f64(&self) -> f64 {
        match *self {
            Coordinate::CoordinateU8(v) => f64::from(v),
            Coordinate::CoordinateU16(v) => f64::from(v),
            Coordinate::CoordinateU32(v) => f64::from(v),
            Coordinate::CoordinateU64(v) => v as f64,
            Coordinate::CoordinateF64(v) => v,
        }
    }

    pub fn u64(&self) -> u64 {
        match *self {
            Coordinate::CoordinateU8(v) => u64::from(v),
            Coordinate::CoordinateU16(v) => u64::from(v),
            Coordinate::CoordinateU32(v) => u64::from(v),
            Coordinate::CoordinateU64(v) => v,
            Coordinate::CoordinateF64(_v) => unreachable!(),
        }
    }
}

/*
impl Serialize for Coordinate {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Coordinate::CoordinateF64(v) => serializer.serialize_f64(*v),
            Coordinate::CoordinateU8(v) => serializer.serialize_u8(*v),
            Coordinate::CoordinateU16(v) => serializer.serialize_u16(*v),
            Coordinate::CoordinateU32(v) => serializer.serialize_u32(*v),
            Coordinate::CoordinateU64(v) => serializer.serialize_u64(*v),
        }
    }
} */

impl Display for Coordinate {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Coordinate::CoordinateU8(v) => write!(f, "{}", v),
            Coordinate::CoordinateU16(v) => write!(f, "{}", v),
            Coordinate::CoordinateU32(v) => write!(f, "{}", v),
            Coordinate::CoordinateU64(v) => write!(f, "{}", v),
            Coordinate::CoordinateF64(v) => write!(f, "{}", v),
        }
    }
}

impl Add<f64> for Coordinate {
    type Output = f64;

    fn add(self, rhs: f64) -> Self::Output {
        self.f64() + rhs
    }
}

impl Add for Coordinate {
    type Output = Coordinate;

    fn add(self, rhs: Self) -> Self::Output {
        if let Coordinate::CoordinateF64(u) = self {
            return Coordinate::CoordinateF64(u + rhs.f64());
        }

        if let Coordinate::CoordinateF64(v) = rhs {
            return Coordinate::CoordinateF64(v + self.f64());
        }

        (self.u64() + rhs.u64()).into()
    }
}

impl Sub<f64> for Coordinate {
    type Output = f64;

    fn sub(self, rhs: f64) -> Self::Output {
        self.f64() - rhs
    }
}

impl Sub for Coordinate {
    type Output = Coordinate;

    fn sub(self, rhs: Self) -> Self::Output {
        if let Coordinate::CoordinateF64(u) = self {
            return Coordinate::CoordinateF64(u - rhs.f64());
        }

        if let Coordinate::CoordinateF64(v) = rhs {
            return Coordinate::CoordinateF64(v - self.f64());
        }
        let r = rhs.u64();
        let l = self.u64();
        let d = if l < r { 0u64 } else { l - r };
        d.into()
    }
}

impl Mul<f64> for Coordinate {
    type Output = Coordinate;

    fn mul(self, rhs: f64) -> Self::Output {
        (self.f64() * rhs).into()
    }
}

impl Mul for Coordinate {
    type Output = Coordinate;

    fn mul(self, rhs: Coordinate) -> Self::Output {
        if let Coordinate::CoordinateF64(u) = self {
            return Coordinate::CoordinateF64(u * rhs.f64());
        }

        if let Coordinate::CoordinateF64(v) = rhs {
            return Coordinate::CoordinateF64(v * self.f64());
        }

        (self.u64() * rhs.u64()).into()
    }
}

impl From<Coordinate> for f64 {
    fn from(v: Coordinate) -> Self {
        v.f64()
    }
}

impl From<&Coordinate> for f64 {
    fn from(v: &Coordinate) -> Self {
        v.f64()
    }
}

impl From<f64> for Coordinate {
    fn from(v: f64) -> Self {
        Coordinate::CoordinateF64(v)
    }
}

impl From<Coordinate> for u64 {
    fn from(v: Coordinate) -> Self {
        v.u64()
    }
}

impl From<&Coordinate> for u64 {
    fn from(v: &Coordinate) -> Self {
        v.u64()
    }
}

impl From<u64> for Coordinate {
    fn from(v: u64) -> Self {
        // Slight syntax hack, as exclusive ranges are not yet available.
        // cf: https://github.com/rust-lang/rust/issues/37854
        match v {
            _ if v <= u64::from(std::u8::MAX) => Coordinate::CoordinateU8(v as u8),
            _ if v <= u64::from(std::u16::MAX) => Coordinate::CoordinateU16(v as u16),
            _ if v <= u64::from(std::u32::MAX) => Coordinate::CoordinateU32(v as u32),
            _ => Coordinate::CoordinateU64(v as u64),
            /*_ => {
                panic!("Out of range {} > {}", v, std::u64::MAX);
            } */
        }
    }
}

impl From<Coordinate> for usize {
    fn from(v: Coordinate) -> Self {
        (v.u64()) as usize
    }
}

impl From<&Coordinate> for usize {
    fn from(v: &Coordinate) -> Self {
        (v.u64()) as usize
    }
}

impl From<usize> for Coordinate {
    fn from(v: usize) -> Self {
        (v as u64).into()
    }
}

impl Ord for Coordinate {
    fn cmp(&self, other: &Self) -> Ordering {
        // If one hand is a floating value, then messy case of floating point
        // values only being partially ordered.
        // TODO: Should we allow comparison between u64 and f64 Coordinates?
        if let Coordinate::CoordinateF64(_lh) = self {
            unimplemented!();
        }

        if let Coordinate::CoordinateF64(_rh) = other {
            unimplemented!();
        }

        self.u64().cmp(&other.u64())
    }
}

impl PartialOrd for Coordinate {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        // If one hand is a floating value, do use floating point comparison,
        // otherwise integer.
        if let Coordinate::CoordinateF64(lh) = self {
            return lh.partial_cmp(&other.f64());
        }

        if let Coordinate::CoordinateF64(rh) = other {
            return self.f64().partial_cmp(rh);
        }

        self.u64().partial_cmp(&other.u64())
    }
}

impl Eq for Coordinate {}

impl PartialEq for Coordinate {
    fn eq(&self, other: &Self) -> bool {
        // If one hand is a floating value, do use floating point comparison,
        // otherwise integer.
        if let Coordinate::CoordinateF64(lh) = self {
            return lh.eq(&other.f64());
        }

        if let Coordinate::CoordinateF64(rh) = other {
            return self.f64().eq(rh);
        }

        self.u64() == other.u64()
    }
}

impl Hash for Coordinate {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Coordinate::CoordinateU8(v) => v.hash(state),
            Coordinate::CoordinateU16(v) => v.hash(state),
            Coordinate::CoordinateU32(v) => v.hash(state),
            Coordinate::CoordinateU64(v) => v.hash(state),
            // FIXME: Ugly workaround... 16 decimal position is enough to
            //        represent any mantissa of 2^53 bits.
            Coordinate::CoordinateF64(v) => format!("{:.*}", 16, v).hash(state),
        }
    }

    /*
    fn hash_slice<H: Hasher>(data: &[Self], state: &mut H) where Self: Sized {
        unimplemented!()
    }*/
}
