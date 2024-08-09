use std::cmp::Ord;

use ironsea_index::IndexedDestructured;
use serde::Deserialize;
use serde::Serialize;

use super::space::Coordinate;
use super::space::Position;
use super::space::Shape;
use super::IterPositions;

#[derive(Clone, Debug, Hash)]
pub struct SpaceSetObject {
    space_id: String,
    position: Position,
    value: usize,
}

impl SpaceSetObject {
    pub fn new(reference_space: &str, position: Position, value: usize) -> Self {
        SpaceSetObject {
            space_id: reference_space.into(),
            position,
            value,
        }
    }

    pub fn space_id(&self) -> &String {
        &self.space_id
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    pub fn value(&self) -> usize {
        self.value
    }

    pub fn set_value(&mut self, value: usize) {
        self.value = value;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceFields {
    space_id: String,
    value: usize,
}

impl SpaceFields {
    pub fn new(space_id: &str, value: usize) -> Self {
        SpaceFields {
            space_id: space_id.into(),
            value,
        }
    }

    pub fn value(&self) -> usize {
        self.value
    }

    pub fn set_value(&mut self, value: usize) {
        self.value = value;
    }
}

impl PartialEq for SpaceFields {
    fn eq(&self, other: &Self) -> bool {
        // WARNING: We ignore the spaceID, as we know it will always be true
        // because of our usage of this.

        // This assumption has to be maintained or the test added back.
        //self.value == other.value

        // First compare on the number field (cheap and fast), then do the String comparison.
        // Safety first
        self.value == other.value && self.space_id == other.space_id
    }
}

impl ironsea_index::Record<Position> for &SpaceSetObject {
    fn key(&self) -> Position {
        self.position.clone()
    }
}

impl ironsea_index::RecordFields<SpaceFields> for &SpaceSetObject {
    fn fields(&self) -> SpaceFields {
        SpaceFields {
            space_id: self.space_id().clone(),
            value: self.value,
        }
    }
}

pub type SpaceSetIndex = ironsea_index_sfc_dbc::IndexOwned<SpaceFields, Position, Coordinate>;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceIndex {
    threshold_volume: f64,
    // lookup_ rounds up, so reverse sort of the list on thresholds and check for last index.
    scale: Vec<u32>,
    index: SpaceSetIndex,
}

impl SpaceIndex {
    pub fn new(threshold_volume: f64, scale: Vec<u32>, index: SpaceSetIndex) -> Self {
        SpaceIndex {
            threshold_volume,
            scale,
            index,
        }
    }

    pub fn threshold(&self) -> f64 {
        self.threshold_volume
    }

    pub fn scale(&self) -> &Vec<u32> {
        &self.scale
    }

    // Inputs and Results are expressed in encoded space coordinates.
    pub fn find<'s>(&'s self, key: &Position) -> Box<dyn Iterator<Item = &SpaceFields> + 's> {
        self.index.find(key)
    }

    // Inputs and Results are expressed in encoded space coordinates.
    fn find_range<'s>(
        &'s self,
        start: &Position,
        end: &Position,
    ) -> Box<dyn Iterator<Item = (Position, &SpaceFields)> + 's> {
        self.index.find_range(start, end)
    }

    // Inputs and Results are expressed in encoded space coordinates.
    pub fn find_by_value<'s>(&'s self, id: &'s SpaceFields) -> IterPositions<'s> {
        self.index.find_by_value(id)
    }

    // Inputs and Results are also in encoded space coordinates.
    pub fn find_by_shape<'s>(
        &'s self,
        shape: Shape,
        view_port: &Option<Shape>,
    ) -> Result<Box<dyn Iterator<Item = (Position, &SpaceFields)> + 's>, String> {
        match shape {
            Shape::Point(position) => {
                if let Some(mbb) = view_port {
                    if !mbb.contains(&position) {
                        return Err(format!(
                            "View port '{:?}' does not contain '{:?}'",
                            mbb, position
                        ));
                    }
                }
                Ok(Box::new(
                    self.find(&position)
                        .map(move |fields| (position.clone(), fields)),
                ))
            }
            Shape::BoundingBox(bl, bh) => {
                if let Some(mbb) = view_port {
                    match mbb {
                        Shape::BoundingBox(vl, vh) => {
                            // Compute the intersection of the two boxes.
                            let lower = (&bl).max(vl);
                            let higher = (&bh).min(vh);
                            if higher < lower {
                                Err(format!(
                                    "View port '{:?}' does not intersect '{:?}'",
                                    mbb,
                                    Shape::BoundingBox(bl.clone(), bh.clone())
                                ))
                            } else {
                                trace!(
                                    "mbb {:?} shape {:?} lower {:?} higher {:?}",
                                    mbb,
                                    Shape::BoundingBox(bl.clone(), bh.clone()),
                                    lower,
                                    higher
                                );
                                Ok(self.find_range(lower, higher))
                            }
                        }
                        _ => Err(format!("Invalid view port shape '{:?}'", mbb)),
                    }
                } else {
                    Ok(self.find_range(&bl, &bh))
                }
            }
            Shape::HyperSphere(center, radius) => {
                let (bl, bh) = Shape::HyperSphere(center.clone(), radius).get_mbb();
                let lower;
                let higher;

                if let Some(mbb) = view_port {
                    match mbb {
                        Shape::BoundingBox(vl, vh) => {
                            // Compute the intersection of the two boxes.
                            lower = (&bl).max(vl);
                            higher = (&bh).min(vh);
                        }
                        _ => return Err(format!("Invalid view port shape '{:?}'", mbb)),
                    }
                } else {
                    lower = &bl;
                    higher = &bh;
                }

                // Filter out results using using a range query over the MBB,
                // then add the condition of the radius as we are working within
                // a sphere.
                let results = self
                    .find_range(lower, higher)
                    .filter(move |(position, _)| (position - &center).norm() <= radius.f64());

                Ok(Box::new(results))
            }
        }
    }
}
