use std::{
    collections::{HashSet, hash_set},
    fmt::Display,
    hash::{Hash, Hasher},
};

use itertools::Itertools;

use super::literal::BooleanVariable;
use crate::{
    intrinsics::{HashUnordered, hashset_partial_order},
    ir::TryMapValues,
};

/// A conjunction of literals.
///
/// `BranchGuard` is the conjunction carried by a conditional CFG edge. An
/// empty guard represents `⊤`.
#[derive(Debug, Clone)]
pub struct BranchGuard<P>(pub(super) HashSet<BooleanVariable<P>>);

impl<P> PartialEq for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}

impl<P> Eq for BranchGuard<P> where P: Hash + Eq {}

impl<P> Hash for BranchGuard<P>
where
    P: Hash,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        (&self.0).hash_unordered(state);
    }
}

impl<P> PartialOrd for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        hashset_partial_order(&self.0, &other.0)
    }
}

impl<P> BranchGuard<P> {
    /// Creates the tautological guard `⊤`.
    #[must_use]
    pub fn one() -> Self {
        Self(HashSet::new())
    }

    /// Creates a guard containing a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self(HashSet::from([predicate]))
    }

    /// Returns whether this guard is `⊤`.
    #[must_use]
    pub fn is_tautology(&self) -> bool {
        self.0.is_empty()
    }

    /// Returns the number of unique predicates referenced by this guard. For estimating the complexity of the path condition.
    #[must_use]
    pub fn predicate_count(&self) -> usize
    where
        P: Hash + Eq,
    {
        self.0
            .iter()
            .map(|it| match it {
                BooleanVariable::Negative(predicate) | BooleanVariable::Positive(predicate) => {
                    predicate
                }
            })
            .unique()
            .count()
    }

    pub(crate) fn predicates(&self) -> impl Iterator<Item = &P> {
        self.0.iter().map(|literal| match literal {
            BooleanVariable::Positive(predicate) | BooleanVariable::Negative(predicate) => {
                predicate
            }
        })
    }

    /// Borrows the predicates while preserving the conjunction structure.
    pub(super) fn as_ref(&self) -> BranchGuard<&P>
    where
        P: Hash + Eq,
    {
        self.0
            .iter()
            .map(|literal| match literal {
                BooleanVariable::Positive(predicate) => BooleanVariable::Positive(predicate),
                BooleanVariable::Negative(predicate) => BooleanVariable::Negative(predicate),
            })
            .collect()
    }
}

impl<P: Display> Display for BranchGuard<P> {
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

impl<P> FromIterator<BooleanVariable<P>> for BranchGuard<P>
where
    P: Hash + Eq,
{
    fn from_iter<T: IntoIterator<Item = BooleanVariable<P>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<P> IntoIterator for BranchGuard<P> {
    type Item = BooleanVariable<P>;
    type IntoIter = hash_set::IntoIter<BooleanVariable<P>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<P, OUT> TryMapValues<OUT> for BranchGuard<P>
where
    P: TryMapValues<OUT>,
    P::Mapped: Eq + Hash,
{
    type Value = P::Value;
    type Mapped = BranchGuard<P::Mapped>;

    fn try_map_values<E>(
        self,
        mut remap: impl FnMut(P::Value) -> Result<OUT, E>,
    ) -> Result<BranchGuard<P::Mapped>, E> {
        Ok(BranchGuard(
            self.0
                .into_iter()
                .map(|literal| literal.try_map_values(&mut remap))
                .collect::<Result<_, E>>()?,
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{TryMapValues, expression::Condition};

    #[test]
    fn display_orders_literals_stably() {
        let lhs = BranchGuard::from_iter([
            BooleanVariable::Positive(2_u8),
            BooleanVariable::Negative(1_u8),
        ]);
        let rhs = BranchGuard::from_iter([
            BooleanVariable::Negative(1_u8),
            BooleanVariable::Positive(2_u8),
        ]);

        assert_eq!(lhs.to_string(), rhs.to_string());
    }

    #[test]
    fn maps_values_and_preserves_literal_polarity() {
        let guard = BranchGuard::from_iter([
            BooleanVariable::Positive(Condition::IsZero(1_u8)),
            BooleanVariable::Negative(Condition::IsNull(2)),
        ]);

        assert_eq!(
            guard.try_map_values(|value| Ok::<_, ()>(u16::from(value) + 10)),
            Ok(BranchGuard::from_iter([
                BooleanVariable::Positive(Condition::IsZero(11_u16)),
                BooleanVariable::Negative(Condition::IsNull(12)),
            ]))
        );
    }
}
