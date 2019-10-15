use ironsea_index::IndexedOwned;
use ironsea_table_vector::VectorTable;

use super::space::Coordinate;
use super::space::Position;
use super::space::Shape;
use super::SpaceId;

#[derive(Clone, Debug, Deserialize, Hash, Serialize)]
pub struct SpaceSetObject {
    space_id: SpaceId,
    position: Position,
    value: Coordinate, // Efficiently store the offset within the SpaceDB values vector
}

impl SpaceSetObject {
    pub fn new(reference_space: &str, position: Position, value: Coordinate) -> Self {
        SpaceSetObject {
            space_id: reference_space.into(),
            position,
            value,
        }
    }

    pub fn id(&self) -> &Coordinate {
        &self.value
    }

    pub fn space_id(&self) -> &SpaceId {
        &self.space_id
    }

    pub fn position(&self) -> &Position {
        &self.position
    }

    pub fn set_position(&mut self, pos: Position) {
        self.position = pos;
    }

    pub fn value(&self) -> &Coordinate {
        &self.value
    }

    pub fn set_value(&mut self, value: Coordinate) {
        self.value = value;
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceFields {
    space_id: SpaceId,
    value: Coordinate,
}

impl SpaceFields {
    pub fn new(space_id: SpaceId, value: Coordinate) -> Self {
        SpaceFields { space_id, value }
    }
}

impl PartialEq for SpaceFields {
    fn eq(&self, other: &Self) -> bool {
        self.space_id == other.space_id && self.value == other.value
    }
}

impl ironsea_index::Record<Position> for SpaceSetObject {
    fn key(&self) -> Position {
        self.position.clone()
    }
}

impl ironsea_index::RecordFields<SpaceFields> for SpaceSetObject {
    fn fields(&self) -> SpaceFields {
        SpaceFields {
            space_id: self.space_id().clone(),
            value: self.value,
        }
    }
}

impl ironsea_index::RecordBuild<Position, SpaceFields, SpaceSetObject> for SpaceSetObject {
    fn build(key: &Position, fields: &SpaceFields) -> SpaceSetObject {
        SpaceSetObject {
            space_id: fields.space_id.clone(),
            position: key.clone(),
            value: fields.value,
        }
    }
}

pub type SpaceSetIndex = ironsea_index_sfc_dbc::IndexOwned<
    VectorTable<SpaceSetObject>,
    SpaceSetObject,
    Position,
    Coordinate,
    SpaceFields,
>;

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

    pub fn find(&self, key: &Position) -> Vec<SpaceSetObject> {
        self.index.find(key)
    }

    fn find_range(&self, start: &Position, end: &Position) -> Vec<SpaceSetObject> {
        self.index.find_range(start, end)
    }

    pub fn find_by_value(&self, id: &SpaceFields) -> Vec<SpaceSetObject> {
        self.index.find_by_value(id)
    }

    // The shape provided in arguments needs to be expressed in encoded space positions.
    // Results are also in encoded space coordinates.
    pub fn find_by_shape(&self, shape: &Shape) -> Result<Vec<SpaceSetObject>, String> {
        match shape {
            Shape::Point(position) => Ok(self.find(position)),
            Shape::BoundingBox(lower, higher) => Ok(self.find_range(lower, higher)),
            Shape::HyperSphere(center, radius) => {
                let (lower, higher) = shape.get_mbb();

                // Filter out results using using a range query over the MBB,
                // then add the condition of the radius as we are working within
                // a sphere.
                let results = self
                    .find_range(&lower, &higher)
                    .into_iter()
                    .filter(|p| (p.position().clone() - center.clone()).norm() <= radius.f64())
                    .collect();

                Ok(results)
            }
        }
    }
}
