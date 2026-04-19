use std::{collections::HashSet, hash::Hash};

mod exact;
mod heuristic;
mod indexed;

use exact::exact_minimize;
use heuristic::heuristic_minimize;
use indexed::AtomTable;

use super::{PathConditionBudget, cube::Cube};

/// A reduction strategy for boolean covers.
pub(super) trait Minimizer<P> {
    /// Returns an equivalent set of cubes with redundant terms removed.
    fn minimize(&self, cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
    where
        P: Hash + Eq + Clone;
}

/// A bounded boolean minimizer for path-condition covers.
#[derive(Debug, Clone, Copy)]
pub(super) struct BoundedMinimizer {
    budget: PathConditionBudget,
}

impl BoundedMinimizer {
    /// Creates a new minimizer with the given resource budget.
    #[must_use]
    pub const fn new(budget: PathConditionBudget) -> Self {
        Self { budget }
    }
}

impl<P> Minimizer<P> for BoundedMinimizer {
    fn minimize(&self, cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
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
            Some(upper_bound) if upper_bound <= self.budget.max_exact_on_set_size => {
                exact_minimize(&cubes, &atoms)
            }
            _ => heuristic_minimize(&cubes, &atoms, self.budget),
        }
    }
}

pub(super) fn absorb<P>(cubes: HashSet<Cube<P>>) -> HashSet<Cube<P>>
where
    P: Hash + Eq,
{
    let mut reduced = HashSet::new();
    for cube in cubes {
        if reduced
            .iter()
            .any(|existing: &Cube<P>| existing.subsumes(&cube))
        {
            continue;
        }

        reduced.retain(|existing| !cube.subsumes(existing));
        reduced.insert(cube);
    }
    reduced
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::control_flow::path_condition::BooleanVariable;

    #[test]
    fn bounded_minimizer_reduces_complementary_terms_exactly() {
        let cubes = HashSet::from([
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Positive(2_u8))
                .unwrap(),
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Negative(2_u8))
                .unwrap(),
        ]);

        let reduced = BoundedMinimizer::new(PathConditionBudget::default()).minimize(cubes);
        assert_eq!(
            reduced,
            HashSet::from([Cube::of(BooleanVariable::Positive(1_u8))])
        );
    }

    #[test]
    fn bounded_minimizer_reduces_complementary_terms_heuristically() {
        let cubes = HashSet::from([
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Positive(2_u8))
                .unwrap(),
            Cube::of(BooleanVariable::Positive(1_u8))
                .conjoin_literal(BooleanVariable::Negative(2_u8))
                .unwrap(),
        ]);

        let reduced = BoundedMinimizer::new(PathConditionBudget {
            max_exact_on_set_size: 0,
            max_heuristic_rounds: 2,
            max_cover_checks: 128,
        })
        .minimize(cubes);
        assert_eq!(
            reduced,
            HashSet::from([Cube::of(BooleanVariable::Positive(1_u8))])
        );
    }

    #[test]
    fn exact_on_set_upper_bound_detects_large_expansions() {
        let cubes = (0_u8..10)
            .map(|predicate| Cube::of(BooleanVariable::Positive(predicate)))
            .collect::<HashSet<_>>();

        assert_eq!(exact_on_set_upper_bound(&cubes, 10), Some(10 * (1 << 9)));
    }
}
