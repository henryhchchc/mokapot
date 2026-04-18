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
    minimizer::{AbsorptionMinimizer, Minimizer},
};
use crate::{
    analysis::fixed_point::JoinSemiLattice,
    intrinsics::{HashUnordered, hashset_partial_order},
};

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
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        hashset_partial_order(&self.cubes, &other.cubes)
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
        let mut cover = Self::zero();
        let minimizer = AbsorptionMinimizer;
        for minterm in minterms {
            if let Some(cube) = Cube::try_from_minterm(&minterm).map(|cube| cube.cloned()) {
                minimizer.insert_cube(&mut cover.cubes, cube);
            }
        }
        cover
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

    fn disjoin(mut self, rhs: Self) -> Self
    where
        P: Hash + Eq,
    {
        let minimizer = AbsorptionMinimizer;
        for cube in rhs.cubes {
            minimizer.insert_cube(&mut self.cubes, cube);
        }
        self
    }

    fn conjoin_literal(self, literal: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        let minimizer = AbsorptionMinimizer;
        let mut cubes = HashSet::new();
        for cube in self.cubes {
            if let Some(cube) = cube.conjoin_literal(literal.clone()) {
                minimizer.insert_cube(&mut cubes, cube);
            }
        }
        Self { cubes }
    }

    fn conjoin(self, rhs: Self) -> Self
    where
        P: Hash + Eq + Clone,
    {
        if self.is_contradiction() || rhs.is_contradiction() {
            return Self::zero();
        }

        let minimizer = AbsorptionMinimizer;
        let mut cubes = HashSet::new();
        for lhs_cube in &self.cubes {
            for rhs_cube in &rhs.cubes {
                if let Some(cube) = lhs_cube.conjoin(rhs_cube) {
                    minimizer.insert_cube(&mut cubes, cube);
                }
            }
        }
        Self { cubes }
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
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.cover.partial_cmp(&other.cover)
    }
}

impl<P> JoinSemiLattice for PathCondition<P>
where
    P: Hash + Eq,
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
    P: Hash + Eq,
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
            cover: self.cover.conjoin_literal(rhs),
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
            cover: self.cover.conjoin(rhs.cover),
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
            write!(f, "{}", self.cover.cubes().format(" || "))
        }
    }
}
