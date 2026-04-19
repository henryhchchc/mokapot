use std::{
    collections::{HashMap, HashSet},
    hash::{Hash, Hasher},
};

use itertools::Itertools;

use super::{
    BooleanVariable, BranchGuard, PathConditionBudget,
    cube::Cube,
    minimizer::{BoundedMinimizer, Minimizer},
};
use crate::intrinsics::HashUnordered;

#[derive(Debug, Clone, Default)]
pub(super) struct Cover<P> {
    cubes: HashSet<Cube<P>>,
}

impl<P> PartialEq for Cover<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.cubes == other.cubes
    }
}

impl<P> Eq for Cover<P> where P: Hash + Eq {}

impl<P> Hash for Cover<P>
where
    P: Hash + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.cubes).hash_unordered(state);
    }
}

impl<P> PartialOrd for Cover<P>
where
    P: Hash + Eq + Clone,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.cubes == other.cubes {
            return Some(std::cmp::Ordering::Equal);
        }

        match (self.implies(other), other.implies(self)) {
            (true, true) => Some(std::cmp::Ordering::Equal),
            (true, false) => Some(std::cmp::Ordering::Less),
            (false, true) => Some(std::cmp::Ordering::Greater),
            (false, false) => None,
        }
    }
}

impl<P> Cover<P> {
    pub(super) fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cubes: HashSet::from([Cube::one()]),
        }
    }

    pub(super) fn zero() -> Self {
        Self {
            cubes: HashSet::new(),
        }
    }

    pub(super) fn of_literal(literal: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cubes: HashSet::from([Cube::of(literal)]),
        }
    }

    #[cfg(test)]
    pub(super) fn from_branch_guards(
        branch_guards: impl IntoIterator<Item = BranchGuard<P>>,
    ) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let cubes = branch_guards
            .into_iter()
            .filter_map(Cube::from_branch_guard)
            .collect();
        Self { cubes }
    }

    pub(super) fn predicates(&self) -> impl Iterator<Item = &P> {
        self.cubes.iter().flat_map(Cube::predicates)
    }

    pub(super) fn cubes(&self) -> impl Iterator<Item = &Cube<P>> {
        self.cubes.iter()
    }

    pub(super) fn is_contradiction(&self) -> bool {
        self.cubes.is_empty()
    }

    fn implies(&self, other: &Self) -> bool
    where
        P: Hash + Eq + Clone,
    {
        let mut memo = HashMap::new();
        self.cubes
            .iter()
            .all(|cube| other.covers_cube(cube, &mut memo))
    }

    fn covers_cube(&self, cube: &Cube<P>, memo: &mut HashMap<Cube<P>, bool>) -> bool
    where
        P: Hash + Eq + Clone,
    {
        if let Some(&result) = memo.get(cube) {
            return result;
        }

        let result = self.covers_cube_uncached(cube, memo);
        memo.insert(cube.clone(), result);
        result
    }

    fn covers_cube_uncached(&self, cube: &Cube<P>, memo: &mut HashMap<Cube<P>, bool>) -> bool
    where
        P: Hash + Eq + Clone,
    {
        if self.cubes.iter().any(|existing| existing.subsumes(cube)) {
            return true;
        }

        let split_predicate = self
            .cubes
            .iter()
            .filter(|existing| !existing.conflicts_with(cube))
            .flat_map(Cube::predicates)
            .find(|predicate| !cube.contains_predicate(predicate))
            .cloned();
        let Some(split_predicate) = split_predicate else {
            return false;
        };

        // A cover contains `cube` only if it contains both refinements of an
        // unconstrained predicate.
        let positive_branch = cube
            .clone()
            .conjoin_literal(BooleanVariable::Positive(split_predicate.clone()));
        let negative_branch = cube
            .clone()
            .conjoin_literal(BooleanVariable::Negative(split_predicate));

        match (positive_branch, negative_branch) {
            (Some(positive_branch), Some(negative_branch)) => {
                self.covers_cube(&positive_branch, memo) && self.covers_cube(&negative_branch, memo)
            }
            _ => false,
        }
    }

    pub(super) fn disjoin(mut self, rhs: Self) -> Self
    where
        P: Hash + Eq,
    {
        for cube in rhs.cubes {
            self.cubes.insert(cube);
        }
        self
    }

    pub(super) fn conjoin_literal(self, literal: &BooleanVariable<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let cubes = self
            .cubes
            .into_iter()
            .filter_map(|cube| cube.conjoin_literal(literal.clone()))
            .collect();
        Self { cubes }
    }

    pub(super) fn conjoin_branch_guard(self, branch_guard: BranchGuard<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        if self.is_contradiction() {
            return Self::zero();
        }

        let Some(rhs_cube) = Cube::from_branch_guard(branch_guard) else {
            return Self::zero();
        };
        if rhs_cube.is_tautology() {
            return self;
        }

        let cubes = self
            .cubes
            .into_iter()
            .filter_map(|lhs_cube| lhs_cube.conjoin(&rhs_cube))
            .collect();
        Self { cubes }
    }

    pub(super) fn conjoin(self, rhs: &Self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        if self.is_contradiction() || rhs.is_contradiction() {
            return Self::zero();
        }

        let cubes = self
            .cubes
            .iter()
            .cartesian_product(&rhs.cubes)
            .filter_map(|(lhs_cube, rhs_cube)| lhs_cube.conjoin(rhs_cube))
            .collect();
        Self { cubes }
    }

    pub(super) fn reduce(self, budget: PathConditionBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let minimizer = BoundedMinimizer::new(budget);
        Self {
            cubes: minimizer.minimize(self.cubes),
        }
    }
}
