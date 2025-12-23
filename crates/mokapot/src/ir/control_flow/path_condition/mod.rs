//! Path constraint analysis.
//!
//! This module implements path condition analysis using disjunctive normal form (DNF).
//! A path condition represents a boolean formula that must be satisfied for a path to be taken.
use std::{
    collections::{HashSet, hash_set},
    fmt::Display,
    hash::Hash,
    ops::{BitAnd, BitOr, Not},
};

use itertools::Itertools;

mod analyzer;

pub use analyzer::*;

use crate::intrinsics::hash_unordered;

/// Path condition in disjunctive normal form.
///
/// Represents a boolean formula as a disjunction of conjunctions (OR of ANDs).
/// An empty set of minterms represents a contradiction (false).
#[derive(Debug, Clone)]
pub struct PathCondition<P> {
    /// The clauses in the disjunctive normal form.
    minterms: HashSet<MinTerm<P>>,
}

impl<P> PartialEq for PathCondition<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.minterms == other.minterms
    }
}

impl<P> Eq for PathCondition<P> where P: Hash + Eq {}

impl<P> Hash for PathCondition<P>
where
    P: Hash + Eq,
{
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_unordered(self.minterms.iter(), state);
    }
}

/// A conjunction of predicates (a minterm in DNF).
///
/// Represents a conjunction (AND) of boolean variables.
/// An empty set of variables represents a tautology (true).
#[derive(Debug, Clone)]
pub struct MinTerm<P>(HashSet<BooleanVariable<P>>);

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
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        hash_unordered(self.0.iter(), state);
    }
}

/// A variable in a path condition.
///
/// Represents either a positive or negative occurrence of a predicate.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
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
    const fn as_ref(&self) -> BooleanVariable<&P> {
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
    pub fn one() -> Self {
        Self(HashSet::new())
    }

    /// Creates a minterm containing a single variable.
    #[must_use]
    pub fn of(pred: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self(HashSet::from([pred]))
    }

    /// Creates a reference to the minterm.
    fn as_ref(&self) -> MinTerm<&P>
    where
        P: Hash + Eq,
    {
        MinTerm(self.0.iter().map(|it| it.as_ref()).collect())
    }

    /// Checks if this minterm contains a variable.
    fn contains(&self, var: &BooleanVariable<P>) -> bool
    where
        P: Hash + Eq,
    {
        self.0.contains(var)
    }

    /// Inserts a variable into this minterm.
    fn insert(&mut self, var: BooleanVariable<P>) -> bool
    where
        P: Hash + Eq,
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

impl<V: Hash + Eq> FromIterator<BooleanVariable<V>> for MinTerm<V> {
    fn from_iter<T: IntoIterator<Item = BooleanVariable<V>>>(iter: T) -> Self {
        Self(iter.into_iter().collect())
    }
}

impl<V> IntoIterator for MinTerm<V> {
    type Item = BooleanVariable<V>;
    type IntoIter = hash_set::IntoIter<BooleanVariable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
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
            minterms: HashSet::from([MinTerm::one()]),
        }
    }

    /// Creates a false value (contradiction).
    #[must_use]
    pub fn zero() -> Self
    where
        P: Hash + Eq,
    {
        Self {
            minterms: HashSet::new(),
        }
    }

    /// Creates a path condition from a single predicate.
    #[must_use]
    pub fn of(pred: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self {
            minterms: HashSet::from([MinTerm::of(pred)]),
        }
    }

    /// Returns a set of variable IDs used in the path condition.
    pub fn predicates(&self) -> HashSet<&P>
    where
        P: Hash + Eq,
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
        P: Hash + Eq,
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
    T: Hash + Eq + Clone,
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
    P: Hash + Eq + Clone,
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
    P: Hash + Eq + Clone,
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
    P: Display + Hash + Eq,
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
    use proptest::{collection::hash_set, prelude::*};

    use super::{BooleanVariable, PathCondition};
    use crate::ir::control_flow::path_condition::MinTerm;

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
        hash_set(
            hash_set(any::<BooleanVariable<u32>>(), 1..26).prop_map(MinTerm),
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
