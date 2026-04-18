use std::{
    collections::HashSet,
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{BitAnd, BitOr},
};

use itertools::Itertools;

#[cfg(test)]
use super::MinTerm;
use super::{
    BooleanVariable,
    cube::Cube,
    minimizer::{ExactMinimizer, Minimizer},
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
            cubes: ExactMinimizer.minimize(self.cubes),
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
    fn from_minterms(minterms: impl IntoIterator<Item = MinTerm<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let cubes = minterms
            .into_iter()
            .filter_map(|minterm| Cube::try_from_minterm(&minterm).map(Cube::cloned))
            .collect();
        Self { cubes }.reduce()
    }

    fn as_ref(&self) -> Cover<&P>
    where
        P: Hash + Eq,
    {
        Cover {
            cubes: self.cubes.iter().map(Cube::as_ref).collect(),
        }
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
        self.cubes.iter().all(|cube| other.covers_cube(cube))
    }

    fn covers_cube(&self, cube: &Cube<P>) -> bool
    where
        P: Hash + Eq + Clone,
    {
        if self.cubes.iter().any(|existing| existing.subsumes(cube)) {
            return true;
        }

        let relevant = self
            .cubes
            .iter()
            .filter(|existing| !existing.conflicts_with(cube))
            .collect::<Vec<_>>();
        if relevant.is_empty() {
            return false;
        }

        let Some(predicate) = relevant
            .iter()
            .flat_map(|existing| existing.predicates())
            .find(|predicate| !cube.contains_predicate(predicate))
            .cloned()
        else {
            return false;
        };

        let positive = cube
            .clone()
            .conjoin_literal(BooleanVariable::Positive(predicate.clone()));
        let negative = cube
            .clone()
            .conjoin_literal(BooleanVariable::Negative(predicate));

        match (positive, negative) {
            (Some(positive), Some(negative)) => {
                self.covers_cube(&positive) && self.covers_cube(&negative)
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

/// Path condition in disjunctive normal form.
///
/// Represents a boolean formula as a disjunction of conjunctions (OR of ANDs).
/// An empty set of minterms represents a contradiction (false).
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
    /// Creates a true value (tautology).
    #[must_use]
    pub fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cover: Cover::one(),
        }
    }

    /// Creates a false value (contradiction).
    #[must_use]
    pub fn zero() -> Self {
        Self {
            cover: Cover::zero(),
        }
    }

    /// Creates a path condition from a single predicate.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self {
            cover: Cover::of_literal(predicate),
        }
    }

    /// Returns a set of variable IDs used in the path condition.
    #[must_use]
    pub fn predicates(&self) -> HashSet<&P>
    where
        P: Hash + Eq,
    {
        self.cover.predicates().collect()
    }

    /// Creates a reference to the path condition.
    pub(super) fn as_ref(&self) -> PathCondition<&P>
    where
        P: Hash + Eq,
    {
        PathCondition {
            cover: self.cover.as_ref(),
        }
    }

    /// Returns true if this path condition is a contradiction (false).
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.cover.is_contradiction()
    }

    #[cfg(test)]
    pub(super) fn from_minterms(minterms: impl IntoIterator<Item = MinTerm<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self {
            cover: Cover::from_minterms(minterms),
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
                .map(|cube| cube.to_string())
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", cubes.iter().format(" || "))
        }
    }
}
