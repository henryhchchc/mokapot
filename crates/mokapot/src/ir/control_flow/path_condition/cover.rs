use std::{
    collections::{HashMap, HashSet},
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{BitAnd, BitOr},
};

use itertools::Itertools;

use super::{
    BooleanVariable, BranchGuard,
    cube::Cube,
    current_budget,
    minimizer::{BoundedMinimizer, Minimizer},
};
use crate::{analysis::fixed_point::JoinSemiLattice, intrinsics::HashUnordered};

#[derive(Debug, Clone, Default)]
struct Cover<P> {
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
    fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cubes: HashSet::from([Cube::one()]),
        }
    }

    fn zero() -> Self {
        Self {
            cubes: HashSet::new(),
        }
    }

    fn reduce(self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self {
            cubes: BoundedMinimizer::new(current_budget()).minimize(self.cubes),
        }
    }

    fn of_literal(literal: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cubes: HashSet::from([Cube::of(literal)]),
        }
    }

    #[cfg(test)]
    fn from_branch_guards(branch_guards: impl IntoIterator<Item = BranchGuard<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let cubes = branch_guards
            .into_iter()
            .filter_map(Cube::from_branch_guard)
            .collect();
        Self { cubes }.reduce()
    }

    fn predicates(&self) -> impl Iterator<Item = &P> {
        self.cubes.iter().flat_map(Cube::predicates)
    }

    fn cubes(&self) -> impl Iterator<Item = &Cube<P>> {
        self.cubes.iter()
    }

    fn is_contradiction(&self) -> bool {
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
        if let Some(result) = memo.get(cube) {
            return *result;
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

    fn disjoin(mut self, rhs: Self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        self.cubes.extend(rhs.cubes);
        self.reduce()
    }

    fn conjoin_literal(self, literal: &BooleanVariable<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let cubes = self
            .cubes
            .into_iter()
            .filter_map(|cube| cube.conjoin_literal(literal.clone()))
            .collect();
        Self { cubes }.reduce()
    }

    fn conjoin_branch_guard(self, branch_guard: BranchGuard<P>) -> Self
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
        Self { cubes }.reduce()
    }

    fn conjoin(self, rhs: &Self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        if self.is_contradiction() || rhs.is_contradiction() {
            return Self::zero();
        }

        let mut cubes = HashSet::new();
        for lhs_cube in &self.cubes {
            for rhs_cube in &rhs.cubes {
                if let Some(cube) = lhs_cube.conjoin(rhs_cube) {
                    cubes.insert(cube);
                }
            }
        }
        Self { cubes }.reduce()
    }
}

/// A reduced path condition in disjunctive normal form.
///
/// `PathCondition` stores a boolean formula as a disjunction of cubes. The
/// internal cover is minimized after conjunctions and disjunctions so equivalent
/// conditions stay compact and compare semantically.
#[derive(Debug, Clone)]
pub struct PathCondition<P> {
    cover: Cover<P>,
}

impl<P> PartialEq for PathCondition<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.cover == other.cover
    }
}

impl<P> Eq for PathCondition<P> where P: Hash + Eq {}

impl<P> Hash for PathCondition<P>
where
    P: Hash + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cover.hash(state);
    }
}

impl<P> PartialOrd for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cover.partial_cmp(&other.cover)
    }
}

impl<P> JoinSemiLattice for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    fn join(self, other: Self) -> Self {
        self | other
    }
}

impl<P> PathCondition<P> {
    /// Creates the tautological condition `⊤`.
    #[must_use]
    pub fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cover: Cover::one(),
        }
    }

    /// Creates the contradictory condition `⊥`.
    #[must_use]
    pub fn zero() -> Self {
        Self {
            cover: Cover::zero(),
        }
    }

    /// Creates a path condition from a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cover: Cover::of_literal(predicate),
        }
    }

    /// Returns the predicates referenced by this condition.
    #[must_use]
    pub fn predicates(&self) -> HashSet<&P>
    where
        P: Hash + Eq,
    {
        self.cover.predicates().collect()
    }

    /// Returns whether this condition is `⊥`.
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.cover.is_contradiction()
    }

    #[cfg(test)]
    pub(super) fn from_branch_guards(
        branch_guards: impl IntoIterator<Item = BranchGuard<P>>,
    ) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self {
            cover: Cover::from_branch_guards(branch_guards),
        }
    }

    #[cfg(test)]
    pub(super) fn cubes(&self) -> impl Iterator<Item = &Cube<P>> {
        self.cover.cubes()
    }
}

impl<P> BitOr for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self {
            cover: self.cover.disjoin(rhs.cover),
        }
    }
}

impl<P> BitAnd<BooleanVariable<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BooleanVariable<P>) -> Self::Output {
        Self {
            cover: self.cover.conjoin_literal(&rhs),
        }
    }
}

impl<P> BitAnd<BranchGuard<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BranchGuard<P>) -> Self::Output {
        Self {
            cover: self.cover.conjoin_branch_guard(rhs),
        }
    }
}

impl<P> BitAnd for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self {
            cover: self.cover.conjoin(&rhs.cover),
        }
    }
}

impl<P> Display for PathCondition<P>
where
    P: Display + Hash + Eq,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_contradiction() {
            write!(f, "⊥")
        } else {
            let cubes = self
                .cover
                .cubes()
                .map(ToString::to_string)
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", cubes.iter().format(" || "))
        }
    }
}
