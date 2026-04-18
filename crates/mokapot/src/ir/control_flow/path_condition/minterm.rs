use std::{
    collections::{HashSet, hash_set},
    fmt::Display,
    hash::{Hash, Hasher},
};

use itertools::Itertools;

use super::literal::BooleanVariable;
use crate::intrinsics::{HashUnordered, hashset_partial_order};

/// A conjunction of literals.
///
/// `MinTerm` is a small public helper for callers that need to talk about a
/// single cube directly. An empty minterm represents `⊤`.
#[derive(Debug, Clone)]
pub struct MinTerm<P>(pub(super) HashSet<BooleanVariable<P>>);

impl<P> PartialEq for MinTerm<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P> Eq for MinTerm<P> where P: Hash + Eq {}

impl<P> Hash for MinTerm<P>
where
    P: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0).hash_unordered(state);
    }
}

impl<P> PartialOrd for MinTerm<P>
where
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        hashset_partial_order(&self.0, &other.0)
    }
}

impl<P> MinTerm<P> {
    /// Creates the tautological minterm `⊤`.
    #[must_use]
    pub fn one() -> Self {
        Self(HashSet::new())
    }

    /// Creates a minterm containing a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self(HashSet::from([predicate]))
    }

    /// Returns an iterator over the literals in the minterm.
    #[cfg(test)]
    pub(super) fn literals(&self) -> impl Iterator<Item = &BooleanVariable<P>> {
        self.0.iter()
    }

    /// Returns whether this minterm is `⊤`.
    #[must_use]
    pub fn is_tautology(&self) -> bool {
        self.0.is_empty()
    }
}

impl<P: Display> Display for MinTerm<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_tautology() {
            write!(f, "⊤")
        } else {
            let literals = self
                .0
                .iter()
                .map(ToString::to_string)
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", literals.iter().format(" && "))
        }
    }
}

impl<P> FromIterator<BooleanVariable<P>> for MinTerm<P>
where
    P: Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = BooleanVariable<P>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<P> IntoIterator for MinTerm<P> {
    type Item = BooleanVariable<P>;
    type IntoIter = hash_set::IntoIter<BooleanVariable<P>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_orders_literals_stably() {
        let lhs = MinTerm::from_iter([
            BooleanVariable::Positive(2_u8),
            BooleanVariable::Negative(1_u8),
        ]);
        let rhs = MinTerm::from_iter([
            BooleanVariable::Negative(1_u8),
            BooleanVariable::Positive(2_u8),
        ]);

        assert_eq!(lhs.to_string(), rhs.to_string());
    }
}
