//! Path constraint analysis.
use std::{
    collections::{BTreeSet, btree_set},
    fmt::Display,
    ops::{BitAnd, BitOr, Not},
};

use itertools::Itertools;

mod analyzer;

pub use analyzer::*;

/// Path condition in disjunctive normal form.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PathCondition<P> {
    /// The clauses in the disjunctive normal form.
    /// An empty set represents a contradiction.
    /// An singleton of an empty set represents a tautology.
    minterms: BTreeSet<MinTerm<P>>,
}

/// A conjunction of predicates.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct MinTerm<P>(BTreeSet<BooleanVariable<P>>);

/// A variable in a path condition.
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
            BooleanVariable::Negative(id) | BooleanVariable::Positive(id) => id,
        }
    }
}

impl<V> Display for BooleanVariable<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BooleanVariable::Positive(id) => write!(f, "({id})"),
            BooleanVariable::Negative(id) => write!(f, "~({id})"),
        }
    }
}

impl<V> Not for BooleanVariable<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            BooleanVariable::Positive(id) => BooleanVariable::Negative(id),
            BooleanVariable::Negative(id) => BooleanVariable::Positive(id),
        }
    }
}

impl<P> MinTerm<P> {
    /// Creates a tautology (i.e., ⊤).
    #[must_use]
    pub const fn one() -> Self {
        Self(BTreeSet::new())
    }

    /// Returns an iterator over the variables in the minterm.
    pub fn iter(&self) -> impl Iterator<Item = &BooleanVariable<P>> {
        self.into_iter()
    }
}

impl<P: Display> Display for MinTerm<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "⊤")
        } else {
            write!(f, "{}", self.0.iter().format(" && "))
        }
    }
}

impl<V: Ord> FromIterator<BooleanVariable<V>> for MinTerm<V> {
    fn from_iter<T: IntoIterator<Item = BooleanVariable<V>>>(iter: T) -> Self {
        let inner = iter.into_iter().collect();
        MinTerm(inner)
    }
}

impl<V> IntoIterator for MinTerm<V> {
    type Item = BooleanVariable<V>;
    type IntoIter = btree_set::IntoIter<BooleanVariable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a MinTerm<V> {
    type Item = &'a BooleanVariable<V>;
    type IntoIter = btree_set::Iter<'a, BooleanVariable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<P> PathCondition<P> {
    /// Creates a true value.
    #[must_use]
    pub fn one() -> Self
    where
        P: Ord,
    {
        let minterms = BTreeSet::from([MinTerm::one()]);
        Self { minterms }
    }

    /// Creates a false value.
    #[must_use]
    pub const fn zero() -> Self
    where
        P: Ord,
    {
        let minterms = BTreeSet::new();
        Self { minterms }
    }

    /// Returns a set of variable IDs used in the SOP.
    pub fn predicates(&self) -> BTreeSet<&P>
    where
        P: Ord,
    {
        self.minterms.iter().flatten().map(BooleanVariable::predicate).collect()
    }
}


impl<V: Ord> FromIterator<MinTerm<V>> for PathCondition<V> {
    fn from_iter<T: IntoIterator<Item = MinTerm<V>>>(iter: T) -> Self {
        let minterms = iter.into_iter().collect();
        Self { minterms }
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
        PathCondition { minterms: products }
    }
}

impl<V> BitAnd for PathCondition<V>
where
    V: Ord + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let PathCondition { minterms: this } = self;
        let PathCondition { minterms: other } = rhs;
        let products = this
            .into_iter()
            .flat_map(|lhs_minterm| {
                other
                    .clone()
                    .into_iter()
                    .filter_map(move |rhs_minterm| {
                        let MinTerm(mut result_inner) = lhs_minterm.clone();
                        for var in rhs_minterm {
                            let neg_var = var.clone().not();
                            if result_inner.contains(&neg_var) {
                                return None;
                            }
                            result_inner.insert(var);
                        }
                        Some(MinTerm(result_inner))
                    })
            })
            .collect();
        PathCondition { minterms: products }
    }
}

impl<T> Display for PathCondition<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PathCondition { minterms: products } = self;
        if products.is_empty() {
            write!(f, "⊥")
        } else {
            write!(f, "{}", products.iter().format(" || "))
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

    use super::{PathCondition, BooleanVariable};

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
