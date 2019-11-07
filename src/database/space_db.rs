use std::cmp::Ordering;
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hash;
use std::hash::Hasher;

use super::space::Position;
use super::space::Shape;
use super::space::Space;
use super::space_index::SpaceFields;
use super::space_index::SpaceIndex;
use super::space_index::SpaceSetIndex;
use super::space_index::SpaceSetObject;
use super::CoreQueryParameters;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct SpaceDB {
    reference_space: String,
    resolutions: Vec<SpaceIndex>,
}

impl SpaceDB {
    pub fn new(
        reference_space: &Space,
        mut space_objects: Vec<SpaceSetObject>,
        scales: Option<Vec<Vec<u32>>>,
        max_elements: Option<usize>,
    ) -> Self {
        //FIXME: Remove hard-coded constants for dimensions & bit length of morton codes.
        const DIMENSIONS: usize = 3;
        const CELL_BITS: usize = 10;

        // Build the set of SpaceIndices.
        let mut resolutions = vec![];
        let mut indices = vec![];

        if let Some(scales) = scales {
            // We optimize scaling, by iteratively building coarser and coarser
            // indexes. Powers holds a list of bit shift to apply based on the
            // previous value.
            let mut powers = Vec::with_capacity(scales.len());

            // Limit temporary values lifetimes
            {
                // Sort by values, smaller to bigger. We clone in order leave as-is scales.
                let mut exps = scales.clone();
                // FIXME: This should be done using all the values, somehow
                exps.sort_unstable_by_key(|v| v[0]);

                let mut previous = 0u32;
                for scale in exps {
                    // FIXME: Remove these assertions ASAP, and support multi-factor scaling
                    assert_eq!(scale.len(), DIMENSIONS);
                    assert!(scale[0] == scale[1] && scale[0] == scale[2]);

                    powers.push((scale[0], scale[0] - previous));
                    previous = scale[0];
                }
            }

            // Apply fixed scales
            let mut count = 0;
            for power in &powers {
                space_objects = space_objects
                    .into_iter()
                    .map(|mut o| {
                        let p = o.position().reduce_precision(power.1);
                        let mut hasher = DefaultHasher::new();
                        o.set_position(p);

                        // Hash, AFTER updating the position.
                        o.hash(&mut hasher);

                        (hasher.finish(), o)
                    })
                    .collect::<HashMap<_, SpaceSetObject>>()
                    .drain()
                    .map(|(_k, v)| v)
                    .collect();

                // Make sure we do not shift more position than available
                let shift = if count >= 31 { 31 } else { count };
                count += 1;
                indices.push((
                    SpaceSetIndex::new(space_objects.iter(), DIMENSIONS, CELL_BITS),
                    vec![power.0, power.0, power.0],
                    shift,
                ));
            }
        } else {
            // Generate scales, following max_elements
            if let Some(max_elements) = max_elements {
                let mut count = 0;

                // The next index should contain at most half the number of
                // elements of the current index.
                let mut element_count_target = space_objects.len() / 2;

                // Insert Full resolution index.
                indices.push((
                    SpaceSetIndex::new(space_objects.iter(), DIMENSIONS, CELL_BITS),
                    vec![count, count, count],
                    0, // Smallest value => highest resolution
                ));

                // Generate coarser indices, until we reach the expect max_element
                // values or we can't define bigger bit shift.
                loop {
                    // Make sure we do not shift more position than available as well.
                    if space_objects.len() <= max_elements || count > 31 {
                        break;
                    }
                    let shift = count;

                    count += 1;
                    space_objects = space_objects
                        .into_iter()
                        .map(|mut o| {
                            let p = o.position().reduce_precision(1);
                            let mut hasher = DefaultHasher::new();
                            o.set_position(p);

                            // Hash, AFTER updating the position.
                            o.hash(&mut hasher);

                            (hasher.finish(), o)
                        })
                        .collect::<HashMap<_, SpaceSetObject>>()
                        .drain()
                        .map(|(_k, v)| v)
                        .collect();

                    // Skip a resolution if it does not bring down enough the
                    // number of points. It would be a waste of space to store it.
                    if element_count_target < space_objects.len() {
                        continue;
                    } else {
                        // The next index should contain at most half the number of
                        // elements of the current index.
                        element_count_target = space_objects.len() / 2;
                    }

                    indices.push((
                        SpaceSetIndex::new(space_objects.iter(), DIMENSIONS, CELL_BITS),
                        vec![count, count, count],
                        shift,
                    ));
                }

            // Generate indices as long as max is smaller than the number of point located in the whole space.
            // For each new index, reduce precision by two, and push to resolutions vectors.
            } else {
                // Generate only full-scale.
                indices.push((
                    SpaceSetIndex::new(space_objects.iter(), DIMENSIONS, CELL_BITS),
                    vec![0, 0, 0],
                    0,
                ));
            }
        }

        // When done, go over the array, and set the threshold_volumes with Volume total / 8 * i in reverse order
        let space_volume = reference_space.volume();
        let max_shift = match indices.last() {
            None => 31,
            Some((_, _, x)) => *x,
        };

        for (index, scale, shift) in indices {
            // Compute threshold volume as Vt = V / 2^(max_shift) * 2^shift
            //  => the smaller shift is, the smaller the threshold is and the higher
            //     the resolution is.
            let volume = space_volume / f64::from(1 << (max_shift - shift));

            resolutions.push(SpaceIndex::new(volume, scale, index));
        }

        // Make sure the vector is sorted by threshold volumes, smallest to largest.
        // this means indices are sorted form highest resolution to lowest resolution.
        // default_resolution() relies on this to find the correct index.
        resolutions.sort_unstable_by(|a, b| match a.threshold().partial_cmp(&b.threshold()) {
            Some(o) => o,
            None => Ordering::Less, // FIXME: This is most likely incorrect...
        });

        SpaceDB {
            reference_space: reference_space.name().clone(),
            resolutions,
        }
    }

    pub fn name(&self) -> &String {
        &self.reference_space
    }

    // The smallest volume threshold, which is the highest resolution,  will
    // be at position 0
    fn highest_resolution(&self) -> usize {
        0
    }

    // The highest volume threshold, which is the lowest resolution,  will
    // be at position len - 1
    fn lowest_resolution(&self) -> usize {
        self.resolutions.len() - 1
    }

    fn resolution_from_volume(&self, volume: f64) -> usize {
        for i in 0..self.resolutions.len() {
            if volume <= self.resolutions[i].threshold() {
                debug!(
                    "Selected {:?} -> {:?} vs {:?}",
                    i,
                    self.resolutions[i].threshold(),
                    volume,
                );

                return i;
            }
        }

        debug!(
            "Selected lowest resolution -> {:?} vs {:?}",
            self.resolutions[self.lowest_resolution()].threshold(),
            volume
        );

        self.lowest_resolution()
    }

    fn resolution_from_scale(&self, scale: &[u32]) -> usize {
        for i in 0..self.resolutions.len() {
            if scale <= self.resolutions[i].scale() {
                debug!(
                    "Selected {:?} -> {:?} vs {:?}",
                    i,
                    self.resolutions[i].scale(),
                    scale
                );

                return i;
            }
        }
        warn!(
            "Scale factors {:?} not found, using lowest resolution: {:?}",
            scale,
            self.resolutions[self.lowest_resolution()].scale()
        );

        self.lowest_resolution()
    }

    // Returns the index to be used by default for the given volume.
    // The index chosen by default will be the one with the smallest volume
    // threshold which is greater or equal to the query volume.
    pub fn resolution(&self, parameters: &CoreQueryParameters) -> usize {
        let CoreQueryParameters {
            threshold_volume,
            resolution,
            ..
        } = parameters;

        // If a specific scale has been set, try to find it, otherwise use the
        // threshold volume to figure a default value, and fall back to the most
        // coarse resolution whenever nothing is specified.
        match resolution {
            None => {
                if let Some(threshold_volume) = threshold_volume {
                    self.resolution_from_volume(*threshold_volume)
                } else {
                    self.lowest_resolution()
                }
            }
            Some(v) => self.resolution_from_scale(v),
        }
    }

    // Search by Id, a.k.a values
    // The results are in encoded space coordinates.
    pub fn get_by_id(
        &self,
        id: usize,
        parameters: &CoreQueryParameters,
    ) -> Result<Vec<(Position)>, String> {
        // Is that ID referenced in the current space?
        let index = self.resolution(parameters);

        // Convert the view port to the encoded space coordinates
        let space = parameters.db.space(&self.reference_space)?;
        let view_port = parameters.view_port(space);

        // Select the objects
        let objects =
            self.resolutions[index].find_by_value(&SpaceFields::new(self.name().into(), id.into()));

        let results = if let Some(view_port) = view_port {
            objects
                .into_iter()
                .filter(|position| view_port.contains(position))
                .collect::<Vec<_>>()
        } else {
            objects
        };

        Ok(results)
    }

    // Search by positions defining a volume.
    // The position is expressed in encoded space coordinates, and results are in encoded space coordinates.
    pub fn get_by_positions(
        &self,
        positions: &[Position],
        parameters: &CoreQueryParameters,
    ) -> Result<Vec<(Position, &SpaceFields)>, String> {
        let index = self.resolution(parameters);

        // FIXME: Should I do it here, or add the assumption this is a clean list?
        // Convert the view port to the encoded space coordinates
        //let space = parameters.db.space(&self.reference_space)?;
        //let view_port = parameters.view_port(space);

        // Select the objects
        let results = positions
            .iter()
            .flat_map(|position| {
                self.resolutions[index]
                    .find(position)
                    .into_iter()
                    .map(move |fields| (position.clone(), fields))
            })
            .collect();

        Ok(results)
    }

    // Search by Shape defining a volume:
    // * Hyperrectangle (MBB),
    // * HyperSphere (radius around a point),
    // * Point (Specific position)

    // The Shape is expressed in encoded space coordinates, and results are in encoded space coordinates.
    pub fn get_by_shape(
        &self,
        shape: &Shape,
        parameters: &CoreQueryParameters,
    ) -> Result<Vec<(Position, &SpaceFields)>, String> {
        let index = self.resolution(parameters);

        // Convert the view port to the encoded space coordinates
        let space = parameters.db.space(&self.reference_space)?;
        let view_port = parameters.view_port(space);

        // Select the objects
        let results = self.resolutions[index].find_by_shape(&shape, &view_port)?;

        Ok(results)
    }
}
