use std::{collections::HashSet, hash::Hash};

use super::cube::Cube;

/// A reduction strategy for boolean covers.
///
/// The current implementation performs cheap normalization only. The trait boundary is
/// intentionally explicit so a future exact minimizer can replace it without changing the
/// public `PathCondition` API.
pub(super) trait Minimizer<P> {
    fn insert_cube(&self, cubes: &mut HashSet<Cube<P>>, cube: Cube<P>)
    where
        P: Hash + Eq;
}

/// The default reduction strategy used by path conditions.
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct AbsorptionMinimizer;

impl<P> Minimizer<P> for AbsorptionMinimizer {
    fn insert_cube(&self, cubes: &mut HashSet<Cube<P>>, cube: Cube<P>)
    where
        P: Hash + Eq,
    {
        if cubes.iter().any(|existing| existing.subsumes(&cube)) {
            return;
        }

        cubes.retain(|existing| !cube.subsumes(existing));
        cubes.insert(cube);
    }
}
