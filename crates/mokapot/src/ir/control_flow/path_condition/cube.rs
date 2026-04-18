use std::{
    cmp::Ordering,
    collections::HashSet,
    fmt::Display,
    hash::{Hash, Hasher},
};

use itertools::Itertools;

use super::{BooleanVariable, BranchGuard};
use crate::intrinsics::{HashUnordered, hashset_partial_order};

/// A normalized conjunction of literals.
///
/// Literals are stored in polarity-specific sets so future minimization passes can
/// reason about positive and negative occurrences independently.
#[derive(Debug, Clone)]
pub(super) struct Cube<P> {
    positive: HashSet<P>,
    negative: HashSet<P>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum InsertResult {
    Present,
    Inserted,
    Contradiction,
}

impl<P> PartialEq for Cube<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.positive == other.positive && self.negative == other.negative
    }
}

impl<P> Eq for Cube<P> where P: Hash + Eq {}

impl<P> Hash for Cube<P>
where
    P: Hash + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u8(0);
        (&self.positive).hash_unordered(state);
        state.write_u8(1);
        (&self.negative).hash_unordered(state);
    }
}

impl<P> PartialOrd for Cube<P>
where
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        product_partial_order(
            hashset_partial_order(&self.positive, &other.positive),
            hashset_partial_order(&self.negative, &other.negative),
        )
    }
}

impl<P> Cube<P> {
    pub(super) fn one() -> Self {
        Self {
            positive: HashSet::new(),
            negative: HashSet::new(),
        }
    }

    pub(super) fn of(literal: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        let mut cube = Self::one();
        let result = cube.insert(literal);
        debug_assert_ne!(result, InsertResult::Contradiction);
        cube
    }

    pub(super) fn from_branch_guard(branch_guard: BranchGuard<P>) -> Option<Self>
    where
        P: Hash + Eq,
    {
        let mut cube = Cube::one();
        for literal in branch_guard {
            if cube.insert(literal) == InsertResult::Contradiction {
                return None;
            }
        }
        Some(cube)
    }

    pub(super) fn predicates(&self) -> impl Iterator<Item = &P> {
        self.positive.iter().chain(self.negative.iter())
    }

    pub(super) fn positive(&self) -> impl Iterator<Item = &P> {
        self.positive.iter()
    }

    pub(super) fn negative(&self) -> impl Iterator<Item = &P> {
        self.negative.iter()
    }

    pub(super) fn literals(&self) -> impl Iterator<Item = BooleanVariable<&P>> {
        self.positive
            .iter()
            .map(BooleanVariable::Positive)
            .chain(self.negative.iter().map(BooleanVariable::Negative))
    }

    pub(super) fn insert(&mut self, literal: BooleanVariable<P>) -> InsertResult
    where
        P: Hash + Eq,
    {
        match literal {
            BooleanVariable::Positive(predicate) => {
                if self.negative.contains(&predicate) {
                    InsertResult::Contradiction
                } else if self.positive.insert(predicate) {
                    InsertResult::Inserted
                } else {
                    InsertResult::Present
                }
            }
            BooleanVariable::Negative(predicate) => {
                if self.positive.contains(&predicate) {
                    InsertResult::Contradiction
                } else if self.negative.insert(predicate) {
                    InsertResult::Inserted
                } else {
                    InsertResult::Present
                }
            }
        }
    }

    pub(super) fn conjoin_literal(mut self, literal: BooleanVariable<P>) -> Option<Self>
    where
        P: Hash + Eq,
    {
        (self.insert(literal) != InsertResult::Contradiction).then_some(self)
    }

    pub(super) fn conjoin(&self, other: &Self) -> Option<Self>
    where
        P: Hash + Eq + Clone,
    {
        if self
            .positive
            .iter()
            .any(|predicate| other.negative.contains(predicate))
            || self
                .negative
                .iter()
                .any(|predicate| other.positive.contains(predicate))
        {
            return None;
        }

        let mut cube = self.clone();
        cube.positive.extend(other.positive.iter().cloned());
        cube.negative.extend(other.negative.iter().cloned());
        Some(cube)
    }

    pub(super) fn subsumes(&self, other: &Self) -> bool
    where
        P: Hash + Eq,
    {
        self.positive.is_subset(&other.positive) && self.negative.is_subset(&other.negative)
    }

    pub(super) fn conflicts_with(&self, other: &Self) -> bool
    where
        P: Hash + Eq,
    {
        self.positive
            .iter()
            .any(|predicate| other.negative.contains(predicate))
            || self
                .negative
                .iter()
                .any(|predicate| other.positive.contains(predicate))
    }

    pub(super) fn contains_predicate(&self, predicate: &P) -> bool
    where
        P: Hash + Eq,
    {
        self.positive.contains(predicate) || self.negative.contains(predicate)
    }

    pub(super) fn is_tautology(&self) -> bool {
        self.positive.is_empty() && self.negative.is_empty()
    }
}

impl<P> Display for Cube<P>
where
    P: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_tautology() {
            write!(f, "⊤")
        } else {
            let literals = self
                .literals()
                .map(|literal| literal.to_string())
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", literals.iter().format(" && "))
        }
    }
}

fn product_partial_order(lhs: Option<Ordering>, rhs: Option<Ordering>) -> Option<Ordering> {
    use Ordering::{Equal, Greater, Less};

    match (lhs?, rhs?) {
        (Equal, ordering) | (ordering, Equal) => Some(ordering),
        (Less, Less) => Some(Less),
        (Greater, Greater) => Some(Greater),
        _ => None,
    }
}
