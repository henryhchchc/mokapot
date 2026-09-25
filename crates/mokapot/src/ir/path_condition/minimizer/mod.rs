use std::{collections::HashSet, hash::Hash};

mod exact;
mod heuristic;
mod indexed;

use exact::exact_minimize;
use heuristic::heuristic_minimize;
use indexed::AtomTable;

use super::{SolvingBudget, cube::Cube};

/// A bounded boolean minimizer for path-condition covers.
#[derive(Debug, Clone, Copy)]
pub(super) struct BoundedMinimizer {
    budget: SolvingBudget,
    generalization: Generalization,
}

/// Whether a [`BoundedMinimizer`] generalizes covers too large for exact
/// minimization.
///
/// Generalization is semantics-preserving, so skipping it cannot move a fixed
/// point; it is where most of a reduction's cost goes, so it is worth running
/// only for reductions whose result is observed.
#[derive(Debug, Clone, Copy)]
enum Generalization {
    /// Run the bounded heuristic reducer.
    Applied,
    /// Return the cover with subsumption and exact minimization only.
    Deferred,
}

impl BoundedMinimizer {
    /// Creates a new minimizer with the given resource budget.
    #[must_use]
    pub const fn new(budget: SolvingBudget) -> Self {
        Self {
            budget,
            generalization: Generalization::Applied,
        }
    }

    /// Creates a minimizer that leaves generalization to its caller.
    #[must_use]
    pub const fn deferring_generalization(budget: SolvingBudget) -> Self {
        Self {
            budget,
            generalization: Generalization::Deferred,
        }
    }

    /// Returns an equivalent set of cubes with redundant terms removed.
    pub(super) fn minimize<P>(&self, cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
    where
        P: Hash + Eq + Clone,
    {
        let cubes = absorb(cubes);
        if cubes.len() <= 1 {
            return cubes;
        }

        let atoms = AtomTable::from_cubes(&cubes);
        let on_set_upper_bound = exact_on_set_upper_bound(&cubes, atoms.len());

        match on_set_upper_bound {
            Some(upper_bound) if upper_bound <= self.budget.on_set_size => {
                exact_minimize(&cubes, &atoms)
            }
            _ => match self.generalization {
                Generalization::Applied => heuristic_minimize(&cubes, &atoms, self.budget),
                Generalization::Deferred => cubes,
            },
        }
    }
}

pub(super) fn absorb<P>(cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
where
    P: Hash + Eq,
{
    cubes.into_iter().fold(HashSet::new(), |mut reduced, cube| {
        if !reduced
            .iter()
            .any(|existing: &Cube<P>| existing.subsumes(&cube))
        {
            reduced.retain(|existing| !cube.subsumes(existing));
            reduced.insert(cube);
        }
        reduced
    })
}

fn exact_on_set_upper_bound<P>(cubes: &HashSet<Cube<P>>, atom_count: usize) -> Option<usize>
where
    P: Hash + Eq,
{
    cubes.iter().try_fold(0usize, |upper_bound, cube| {
        let dont_care_count = atom_count.saturating_sub(cube.predicates().count());
        if dont_care_count >= usize::BITS as usize {
            return None;
        }

        let cube_expansion = 1usize << dont_care_count;
        upper_bound.checked_add(cube_expansion)
    })
}
