//! Path constraint analysis.
//!
//! This module implements path condition analysis using disjunctive normal form (DNF).
//! A path condition represents a boolean formula that must be satisfied for a path to be taken.
use std::{
    cmp::Ord,
    collections::{BTreeSet, btree_set},
    fmt::Display,
    ops::{BitAnd, BitOr, Not},
};

use itertools::Itertools;

mod analyzer;

pub use analyzer::*;

/// Path condition in disjunctive normal form.
///
/// Represents a boolean formula as a disjunction of conjunctions (OR of ANDs).
/// An empty set of minterms represents a contradiction (false).
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PathCondition<P> {
    /// The clauses in the disjunctive normal form.
    minterms: BTreeSet<MinTerm<P>>,
}

/// A conjunction of predicates (a minterm in DNF).
///
/// Represents a conjunction (AND) of boolean variables.
/// An empty set of variables represents a tautology (true).
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct MinTerm<P>(BTreeSet<BooleanVariable<P>>);

/// A variable in a path condition.
///
/// Represents either a positive or negative occurrence of a predicate.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum BooleanVariable<P> {
    /// A positive variable.
    Positive(P),
    /// A negative variable.
    Negative(P),
}

impl<P> BooleanVariable<P> {
    /// Returns a reference to the inner id of the variable.
    pub const fn predicate(&self) -> &P {
        match self {
            Self::Negative(id) | Self::Positive(id) => id,
        }
    }

    /// Creates a reference to the boolean variable.
    fn as_ref(&self) -> BooleanVariable<&P> {
        match self {
            Self::Positive(id) => BooleanVariable::Positive(id),
            Self::Negative(id) => BooleanVariable::Negative(id),
        }
    }
}

impl<V> Display for BooleanVariable<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Positive(id) => write!(f, "({id})"),
            Self::Negative(id) => write!(f, "~({id})"),
        }
    }
}

impl<V> Not for BooleanVariable<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Self::Positive(id) => Self::Negative(id),
            Self::Negative(id) => Self::Positive(id),
        }
    }
}

impl<P> MinTerm<P> {
    /// Creates a tautology (i.e., ⊤).
    #[must_use]
    pub const fn one() -> Self {
        Self(BTreeSet::new())
    }

    /// Creates a minterm containing a single variable.
    #[must_use]
    pub fn of(pred: BooleanVariable<P>) -> Self
    where
        P: Ord,
    {
        Self(BTreeSet::from([pred]))
    }

    /// Creates a reference to the minterm.
    fn as_ref(&self) -> MinTerm<&P>
    where
        P: Ord,
    {
        MinTerm(self.0.iter().map(|it| it.as_ref()).collect())
    }

    /// Checks if this minterm contains a variable.
    fn contains(&self, var: &BooleanVariable<P>) -> bool
    where
        P: Ord,
    {
        self.0.contains(var)
    }

    /// Inserts a variable into this minterm.
    fn insert(&mut self, var: BooleanVariable<P>) -> bool
    where
        P: Ord,
    {
        self.0.insert(var)
    }

    /// Returns true if this minterm is empty (represents a tautology).
    fn is_tautology(&self) -> bool {
        self.0.is_empty()
    }
}

impl<P: Display> Display for MinTerm<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_tautology() {
            write!(f, "⊤")
        } else {
            write!(f, "{}", self.0.iter().format(" && "))
        }
    }
}

impl<V: Ord> FromIterator<BooleanVariable<V>> for MinTerm<V> {
    fn from_iter<T: IntoIterator<Item = BooleanVariable<V>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V> IntoIterator for MinTerm<V> {
    type Item = BooleanVariable<V>;
    type IntoIter = btree_set::IntoIter<BooleanVariable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<P> PathCondition<P> {
    /// Creates a true value (tautology).
    #[must_use]
    pub fn one() -> Self
    where
        P: Ord,
    {
        Self {
            minterms: BTreeSet::from([MinTerm::one()]),
        }
    }

    /// Creates a false value (contradiction).
    #[must_use]
    pub const fn zero() -> Self
    where
        P: Ord,
    {
        Self {
            minterms: BTreeSet::new(),
        }
    }

    /// Creates a path condition from a single predicate.
    #[must_use]
    pub fn of(pred: BooleanVariable<P>) -> Self
    where
        P: Ord,
    {
        Self {
            minterms: BTreeSet::from([MinTerm::of(pred)]),
        }
    }

    /// Returns a set of variable IDs used in the path condition.
    pub fn predicates(&self) -> BTreeSet<&P>
    where
        P: Ord,
    {
        self.minterms
            .iter()
            .flat_map(|it| it.0.iter())
            .map(BooleanVariable::predicate)
            .collect()
    }

    /// Creates a reference to the path condition.
    fn as_ref(&self) -> PathCondition<&P>
    where
        P: Ord,
    {
        PathCondition {
            minterms: self.minterms.iter().map(|it| it.as_ref()).collect(),
        }
    }

    /// Returns true if this path condition is a contradiction (false).
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.minterms.is_empty()
    }
}

impl<T> BitOr for PathCondition<T>
where
    T: Ord + Clone,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut products = self.minterms;
        products.extend(rhs.minterms);
        Self { minterms: products }
    }
}

impl<P> BitAnd<BooleanVariable<P>> for PathCondition<P>
where
    P: Ord + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BooleanVariable<P>) -> Self::Output {
        let minterms = self
            .minterms
            .into_iter()
            .filter_map(|minterm| {
                // Check if the minterm contains the negation of the variable
                // If so, this term becomes false (contradiction) and is dropped
                if minterm.contains(&rhs.clone().not()) {
                    return None;
                }

                // Add the variable to the minterm
                let mut updated_minterm = minterm;
                updated_minterm.insert(rhs.clone());
                Some(updated_minterm)
            })
            .collect();
        Self { minterms }
    }
}

impl<P> BitAnd for PathCondition<P>
where
    P: Ord + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let products = self
            .minterms
            .into_iter()
            .flat_map(|lhs_minterm| {
                rhs.minterms
                    .clone()
                    .into_iter()
                    .filter_map(move |rhs_minterm| {
                        let mut result = lhs_minterm.clone();

                        // Check for contradiction: if any variable appears with opposite signs
                        for var in &rhs_minterm.0 {
                            if result.contains(&var.clone().not()) {
                                return None;
                            }
                        }

                        // Combine the variables
                        for var in rhs_minterm {
                            result.insert(var);
                        }

                        Some(result)
                    })
            })
            .collect();

        Self { minterms: products }
    }
}

impl<P> Display for PathCondition<P>
where
    P: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.is_contradiction() {
            write!(f, "⊥")
        } else {
            write!(f, "{}", self.minterms.iter().format(" || "))
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use itertools::Itertools;
    use proptest::collection::btree_set;
    use proptest::prelude::*;

    use crate::ir::control_flow::path_condition::MinTerm;

    use super::{BooleanVariable, PathCondition};

    impl proptest::arbitrary::Arbitrary for BooleanVariable<u32> {
        type Parameters = (u32, bool);
        type Strategy = Just<Self>;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            let (id, negative) = args;
            if negative {
                Just(BooleanVariable::Negative(id))
            } else {
                Just(BooleanVariable::Positive(id))
            }
        }
    }

    fn evaluate(cond: PathCondition<u32>, value_map: &HashMap<u32, bool>) -> bool {
        cond.minterms
            .into_iter()
            .map(|minterm| {
                minterm.into_iter().all(|ref it| match it {
                    BooleanVariable::Positive(id) => value_map[id],
                    BooleanVariable::Negative(id) => !value_map[id],
                })
            })
            .reduce(|lhs, rhs| lhs || rhs)
            .unwrap_or_default()
    }

    fn generate_pred_values(cond: &PathCondition<u32>) -> HashMap<u32, bool> {
        cond.minterms
            .iter()
            .flat_map(|it| it.0.iter())
            .map(BooleanVariable::predicate)
            .copied()
            .dedup()
            .map(|it| (it, rand::random()))
            .collect()
    }

    fn arb_test_cond() -> impl Strategy<Value = PathCondition<u32>> {
        btree_set(
            btree_set(any::<BooleanVariable<u32>>(), 1..26).prop_map(MinTerm),
            1..26,
        )
        .prop_map(|minterms| PathCondition { minterms })
    }

    proptest! {
        #[test]
        fn and(
            lhs in arb_test_cond(),
            rhs in arb_test_cond()
        ) {
            let mut pred_values = generate_pred_values(&lhs);
            pred_values.extend(generate_pred_values(&rhs));
            let lhs_eval = evaluate(lhs.clone(), &pred_values);
            let rhs_eval = evaluate(rhs.clone(), &pred_values);
            let conjunction = lhs.clone() & rhs.clone();
            let conjunction_eval = evaluate(conjunction.clone(), &pred_values);
            assert_eq!(lhs_eval && rhs_eval, conjunction_eval);
        }

        #[test]
        fn or(
            lhs in arb_test_cond(),
            rhs in arb_test_cond()
        ) {
            let mut pred_values = generate_pred_values(&lhs);
            pred_values.extend(generate_pred_values(&rhs));
            let lhs_eval = evaluate(lhs.clone(), &pred_values);
            let rhs_eval = evaluate(rhs.clone(), &pred_values);
            let disjunction = lhs.clone() | rhs.clone();
            let disjunction_eval = evaluate(disjunction.clone(), &pred_values);
            assert_eq!(lhs_eval || rhs_eval, disjunction_eval);
        }
    }
}
