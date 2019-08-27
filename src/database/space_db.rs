use super::space::Coordinate;
use super::space::Position;
use super::space::Shape;
use super::space_index::SpaceFields;
use super::space_index::SpaceIndex;
use super::space_index::SpaceSetIndex;
use super::space_index::SpaceSetObject;

use ironsea_table_vector::VectorTable;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceDB {
    reference_space: String,
    values: Vec<Coordinate>,
    resolutions: Vec<SpaceIndex>,
}

impl SpaceDB {
    pub fn new<S>(reference_space: S, mut space_objects: Vec<SpaceSetObject>) -> Self
    where
        S: Into<String>,
    {
        let mut values = space_objects
            .iter()
            .map(|object| *object.value())
            .collect::<Vec<_>>();

        values.sort_unstable_by_key(|&c| c.u64());
        values.dedup_by_key(|c| c.u64());

        space_objects.iter_mut().for_each(|object| {
            // Update the values to point into the local (shorter) mapping array.
            let val = values.binary_search(object.value()).unwrap();
            object.set_value(val.into());
        });

        // Build the set of SpaceIndices.
        // FIXME: Build multiple-scale indices. What is the stopping condition, and what are the parameters?
        let max_elem = 2_000;
        // We cannot return less that the total number of individual Ids stored
        // in the index.
        let max = max_elem.max(values.len());
        // Generate indices as long as max is smaller than the number of point located in the whole space.
        // For each new index, reduce precision by two, and push to resolutions vectors.

        // When done, go over the array, and set the threshold_volumes with Volume total / 8 * i in reverse order
        //
        let index = SpaceSetIndex::new(&VectorTable::new(space_objects), 3, 10);
        let mut resolutions = vec![SpaceIndex::new(std::f64::MAX, vec![0, 0, 0], index)];

        // Make sure the vector is sorted by threshold volumes, smallest to largest.
        // this means indices are sorted form highest resolution to lowest resolution.
        // default_resolution() relies on it to find the correct index.
        //FIXME: Domain check between f64 <-> u64 XOR implement Ord on f64
        resolutions.sort_unstable_by_key(|a| a.threshold() as u64);

        SpaceDB {
            reference_space: reference_space.into(),
            values,
            resolutions,
        }
    }

    pub fn name(&self) -> &String {
        &self.reference_space
    }

    // The smallest volume threshold, which is the highest resolution,  will
    // be at position 0
    pub fn highest_resolution(&self) -> usize {
        0
    }

    // The highest volume threshold, which is the lowest resolution,  will
    // be at position len - 1
    pub fn lowest_resolution(&self) -> usize {
        self.resolutions.len() - 1
    }

    // Is this Space DB empty?
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    // Returns the index to be used by default for the given volume.
    // The index chosen by default will be the one with the smallest volume
    // threshold which is greater or equal to the query volume.
    pub fn default_resolution(&self, volume: f64) -> usize {
        for i in 0..self.resolutions.len() {
            if volume <= self.resolutions[i].threshold() {
                return i;
            }
        }
        self.resolutions.len()
    }

    // Convert the value back to caller's references
    fn decode(&self, mut objects: Vec<SpaceSetObject>) -> Vec<SpaceSetObject> {
        for o in &mut objects {
            o.set_value(self.values[o.value().u64() as usize]);
        }

        objects
    }

    // Search by Id, a.k.a values
    pub fn get_by_id(
        &self,
        id: usize,
        threshold_volume: f64,
    ) -> Result<Vec<SpaceSetObject>, String> {
        // Is that ID referenced in the current space?
        if let Ok(offset) = self.values.binary_search(&id.into()) {
            let resolution = self.default_resolution(threshold_volume);

            let mut results = self.resolutions[resolution]
                .find_by_value(&SpaceFields::new(self.name().into(), offset.into()));

            // Convert the Value back to caller's references
            // Here we do not use decode() as we have a single id value to manage.
            for o in &mut results {
                o.set_value(id.into());
            }

            Ok(results)
        } else {
            Ok(vec![])
        }
    }

    // Search by positions defining a volume.
    pub fn get_by_positions(
        &self,
        positions: &[Position],
        threshold_volume: f64,
    ) -> Result<Vec<SpaceSetObject>, String> {
        let resolution = self.default_resolution(threshold_volume);

        let results = positions
            .iter()
            .flat_map(|position| self.resolutions[resolution].find(position))
            .collect::<Vec<SpaceSetObject>>();

        Ok(self.decode(results))
    }

    // Search by Shape defining a volume:
    // * Hyperrectangle (MBB),
    // * HyperSphere (radius around a point),
    // * Point (Specific position)
    pub fn get_by_shape(
        &self,
        shape: &Shape,
        threshold_volume: f64,
    ) -> Result<Vec<SpaceSetObject>, String> {
        let resolution = self.default_resolution(threshold_volume);
        Ok(self.decode(self.resolutions[resolution].find_by_shape(&shape)?))
    }
}
