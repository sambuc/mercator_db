use serde::Deserialize;
use serde::Serialize;

use super::Coordinate;
use super::Position;
use super::Space;

/// Known shapes descriptions
#[derive(Clone, Debug, Deserialize, Serialize)]
pub enum Shape {
    /// A singular point in space.
    Point(Position),
    //HyperRectangle([Position; MAX_K]),
    /// A sphere in space.
    HyperSphere(Position, Coordinate),

    /// Hyperrectangle whose faces have one of the axis as a normal.
    BoundingBox(Position, Position),
    //Nifti(nifti_data??),
}

impl Shape {
    /// Convert the encoded coordinates between two reference spaces.
    ///
    /// The resulting shape is expressed in encoded coordinates in the
    /// target space.
    ///
    /// # Parameters
    ///
    ///  * `from`:
    ///     Current reference space of the shape.
    ///
    ///  * `to`:
    ///     Target reference space.
    pub fn rebase(&self, from: &Space, to: &Space) -> Result<Shape, String> {
        match self {
            Shape::Point(position) => Ok(Shape::Point(Space::change_base(position, from, to)?)),
            Shape::HyperSphere(center, radius) => {
                //FIXME: Is the length properly dealt with? How do we process this for space conversions?
                let mut r = Vec::with_capacity(center.dimensions());
                for _ in 0..center.dimensions() {
                    r.push(radius.clone());
                }
                let r = r.into();
                let r = from.absolute_position(&r)?;
                let r = to.rebase(&(r))?[0];
                Ok(Shape::HyperSphere(Space::change_base(center, from, to)?, r))
            }
            Shape::BoundingBox(lower, higher) => Ok(Shape::BoundingBox(
                Space::change_base(lower, from, to)?,
                Space::change_base(higher, from, to)?,
            )),
        }
    }

    /// Decode the coordinates of the shape.
    ///
    /// The encoded coordinates of the shapes are expressed in the
    /// provided space.
    ///
    /// # Parameters
    ///
    ///  * `space`:
    ///      Reference space of the shape. It is used to decode the
    ///      encoded coordinates into positions.
    ///
    /// # Return value
    ///
    /// The shape with decoded positions within the space.
    pub fn decode(&self, space: &Space) -> Result<Shape, String> {
        let s = match self {
            Shape::Point(position) => Shape::Point(space.decode(position)?.into()),
            Shape::HyperSphere(center, radius) => {
                //FIXME: Is the length properly dealt with? How do we process this for space conversions?
                Shape::HyperSphere(space.decode(center)?.into(), *radius)
            }
            Shape::BoundingBox(lower, higher) => {
                Shape::BoundingBox(space.decode(lower)?.into(), space.decode(higher)?.into())
            }
        };

        Ok(s)
    }

    /// Encode the positions of the shape.
    ///
    /// The positions of the shapes are expressed in the provided space.
    ///
    /// # Parameters
    ///
    ///  * `space`:
    ///      Reference space of the shape. It is used to encode the
    ///      positions into encoded coordinates.
    ///
    /// # Return value
    ///
    /// The shape with encoded coordinates within the space.
    pub fn encode(&self, space: &Space) -> Result<Shape, String> {
        let s = match self {
            Shape::Point(position) => {
                let p: Vec<f64> = position.into();
                Shape::Point(space.encode(&p)?)
            }
            Shape::HyperSphere(center, radius) => {
                let p: Vec<f64> = center.into();
                //FIXME: Is the length properly dealt with? How do we process this for space conversions?
                Shape::HyperSphere(space.encode(&p)?, *radius)
            }
            Shape::BoundingBox(lower, higher) => {
                let lower: Vec<f64> = lower.into();
                let higher: Vec<f64> = higher.into();
                Shape::BoundingBox(space.encode(&lower)?, space.encode(&higher)?)
            }
        };

        Ok(s)
    }

    /// Compute the minimum bounding box of the shape.
    ///
    /// This is an hyperrectangle whose faces are perpendicular to an
    /// axis of the space, and which minimally covers the shape.
    pub fn get_mbb(&self) -> (Position, Position) {
        match self {
            Shape::Point(position) => (position.clone(), position.clone()),
            Shape::HyperSphere(center, radius) => {
                let dimensions = center.dimensions();
                let mut vr = Vec::with_capacity(dimensions);
                for _ in 0..dimensions {
                    vr.push(*radius);
                }
                let vr: &Position = &vr.into();
                (center - vr, center + vr)
            }
            Shape::BoundingBox(lower, higher) => (lower.clone(), higher.clone()),
        }
    }

    /// Check if the shape overlaps with the given position.
    ///
    /// # Parameters
    ///
    ///  * `position`:
    ///      The position to check.
    pub fn contains(&self, position: &Position) -> bool {
        match self {
            Shape::Point(reference) => reference == position,
            Shape::HyperSphere(center, radius) => (position - center).norm() <= radius.f64(),
            Shape::BoundingBox(lower, higher) => lower <= position && position <= higher,
        }
    }

    /* Original version proposed by Charles François Rey - 2019
    ```perl
    use strict;

    my $conf = [[0, 2], [1, 3], [11, 20], [5, 6]];
    my $dim = scalar @{$conf};

    sub nxt {
        my ($state) = @_;
        foreach my $i (0..$dim-1) {
            $i = $dim-1-$i;
            $state->[$i] = $state->[$i] + 1;
            if ($state->[$i] > $conf->[$i]->[-1]) {
                $state->[$i] = $conf->[$i]->[0];
                # => carry
            } else {
                return 1;
            }
        }
        return;
    }

    sub pretty {
        my ($state) = @_;
        return "(", join(', ', @{$state}), ")";
    }

    sub first {
        return [ map { $_->[0] } @{$conf} ];
    }

    my $i = 0;
    my $s = first;
    do {
        print $i++, ": ", pretty($s), "\n";
    } while (nxt($s))
    ```*/
    fn gen(lower: &Position, higher: &Position) -> Vec<Position> {
        fn next(
            dimensions: usize,
            lower: &Position,
            higher: &Position,
            state: &mut Position,
        ) -> bool {
            for i in (0..dimensions).rev() {
                state[i] = (state[i].u64() + 1).into();
                if state[i] >= higher[i] {
                    state[i] = lower[i];
                // => carry
                } else {
                    return true;
                }
            }

            false
        }

        fn first(lower: &Position) -> Position {
            let mut current = vec![];
            for i in 0..lower.dimensions() {
                current.push(lower[i].u64());
            }

            current.into()
        }

        let mut results = vec![];

        // Redefine lower as a compacted form of lower for all coordinates.
        let lower = first(lower);

        // Initialise the current value
        let mut current = lower.clone();

        // Add the first Position to the results, as next will return the following one.
        results.push(current.clone());
        while next(lower.dimensions(), &lower, higher, &mut current) {
            results.push(current.clone())
        }
        results
    }

    /// Transform a Shape into a list of `Position` which approximate
    /// the shape.
    // TODO: Return an iterator instead, for performance!
    pub fn rasterise(&self) -> Result<Vec<Position>, String> {
        match self {
            Shape::Point(position) => Ok(vec![position.clone()]),
            Shape::HyperSphere(center, radius) => {
                let (lower, higher) = self.get_mbb();
                let radius = radius.f64();

                let positions = Shape::gen(&lower, &higher)
                    .into_iter()
                    .filter(|p| (p - center).norm() <= radius)
                    .collect();

                Ok(positions)
            }
            Shape::BoundingBox(lower, higher) => Ok(Shape::gen(lower, higher)),
        }
    }

    /// Transform a Shape into a list of `Position` which approximate
    /// the shape, in absolute, or Universe positions.
    ///
    /// # Parameters
    ///
    ///  * `space`:
    ///      Reference space in which the shape is expressed.
    ///
    // TODO: Return an iterator instead, for performance!
    pub fn rasterise_from(&self, space: &Space) -> Result<Vec<Position>, String> {
        Ok(self
            .rasterise()?
            .into_iter()
            .filter_map(|p| match space.absolute_position(&p) {
                Ok(p) => Some(p),
                Err(_) => None, // Should be impossible, but let's handle the case.
            })
            .collect())
    }

    /// Compute the volume.
    pub fn volume(&self) -> f64 {
        match self {
            Shape::Point(_) => std::f64::EPSILON, // Smallest non-zero volume possible
            Shape::BoundingBox(low, high) => {
                let mut volume = 1.0;

                // For each dimension, multiply by the length in that dimension
                for i in 0..low.dimensions() {
                    let l = low[i].f64();
                    let h = high[i].f64();
                    let length = if h > l { h - l } else { l - h };

                    volume *= length;
                }

                volume
            }
            Shape::HyperSphere(position, radius) => {
                // Formula from https://en.wikipedia.org/wiki/N-sphere#/media/File:N_SpheresVolumeAndSurfaceArea.png
                let k = position.dimensions(); // Number of dimensions.
                let radius = radius.f64();

                let pi = std::f64::consts::PI;
                let factor = 2.0 * pi;

                // Set starting values for the coefficient
                let mut a = 2.0;
                let mut i = if (k % 2) == 0 {
                    a = pi;
                    2
                } else {
                    1
                };

                while i < k {
                    i += 2;
                    a *= factor;
                    a /= i as f64;
                }

                a * radius.powi(i as i32)
            }
        }
    }
}
