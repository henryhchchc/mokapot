//! Path constraint analysis.
use std::{
    collections::{BTreeMap, BTreeSet, btree_set},
    fmt::Display,
    ops::{BitAnd, BitOr, Not},
};

use itertools::Itertools;

mod analyzer;

pub use analyzer::*;

/// Path condition in disjunctive normal form.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct SOP<V> {
    /// The clauses in the disjunctive normal form.
    /// An empry set represents a contradiction.
    /// An singleton of an empty set represents a tautology.
    minterms: BTreeSet<MinTerm<V>>,
}

/// A conjunction of predicates.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub struct MinTerm<V>(BTreeSet<Variable<V>>);

/// A variable in a path condition.
#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone)]
pub enum Variable<ID> {
    /// A positive variable.
    Positive(ID),
    /// A negative variable.
    Negative(ID),
}

impl<ID> Variable<ID> {
    /// Returns a reference to the inner id of the variable.
    pub const fn id(&self) -> &ID {
        match self {
            Variable::Negative(id) | Variable::Positive(id) => id,
        }
    }
}

impl<V> Display for Variable<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Variable::Positive(id) => write!(f, "({id})"),
            Variable::Negative(id) => write!(f, "~({id})"),
        }
    }
}

impl<V> Not for Variable<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        match self {
            Variable::Positive(id) => Variable::Negative(id),
            Variable::Negative(id) => Variable::Positive(id),
        }
    }
}

impl<P> BitAnd for MinTerm<P>
where
    P: Ord,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let MinTerm(mut new) = self;
        let MinTerm(rhs) = rhs;
        new.extend(rhs);
        MinTerm(new)
    }
}

impl<P> MinTerm<P> {
    /// Creates a tautology (i.e., ⊤).
    #[must_use]
    pub fn one() -> Self {
        Self(BTreeSet::default())
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

impl<V: Ord> FromIterator<Variable<V>> for MinTerm<V> {
    fn from_iter<T: IntoIterator<Item = Variable<V>>>(iter: T) -> Self {
        let inner = iter.into_iter().collect();
        MinTerm(inner)
    }
}

impl<V> IntoIterator for MinTerm<V> {
    type Item = Variable<V>;
    type IntoIter = btree_set::IntoIter<Variable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a, V> IntoIterator for &'a MinTerm<V> {
    type Item = &'a Variable<V>;
    type IntoIter = btree_set::Iter<'a, Variable<V>>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl<P> SOP<P> {
    /// Creates a tautology.
    #[must_use]
    pub fn one() -> Self
    where
        P: Ord,
    {
        let products = BTreeSet::from([MinTerm::one()]);
        Self { minterms: products }
    }

    /// Creates a contradiction.
    #[must_use]
    pub fn zero() -> Self
    where
        P: Ord,
    {
        let products = BTreeSet::default();
        Self { minterms: products }
    }

    /// Simplifies the path condition.
    pub fn simplify(&mut self)
    where
        P: Ord + Clone,
    {
        use quine_mccluskey as qmc;
        let variable_ids = self.variable_ids();
        let idx_count = variable_ids.len();
        if variable_ids.len() > qmc::DEFAULT_VARIABLES.len() {
            todo!("Too many variables")
        }
        let var_idx: BTreeMap<_, _> = variable_ids
            .iter()
            .enumerate()
            .map(|(idx, var)| (*var, idx as u32))
            .collect();
        let var_map: BTreeMap<_, _> = qmc::DEFAULT_VARIABLES
            .into_iter()
            .zip(variable_ids)
            .collect();
        let minterms: Vec<_> = self.minterms.iter().map(|it| it.index(&var_idx)).collect();
        let mut solutions = qmc::minimize_minterms(
            &qmc::DEFAULT_VARIABLES[..idx_count],
            &minterms,
            &[],
            true,
            None,
        )
        .expect("There should be a result as no timeout is set.");
        let qmc::Solution::SOP(sop_solution) = solutions
            .pop()
            .expect("There should be at least one solution")
        else {
            unreachable!("We are using minimize_minterms")
        };
        let new_sop = sop_solution
            .into_iter()
            .map(|min_terms| {
                min_terms
                    .into_iter()
                    .map(|var| {
                        let id = (*var_map.get(var.name.as_str()).unwrap()).clone();
                        if var.is_negated {
                            Variable::Negative(id)
                        } else {
                            Variable::Positive(id)
                        }
                    })
                    .collect()
            })
            .collect();

        *self = new_sop;
    }

    fn variable_ids(&self) -> BTreeSet<&P>
    where
        P: Ord,
    {
        self.minterms
            .iter()
            .flat_map(|term| term.into_iter())
            .map(Variable::id)
            .collect()
    }
}

impl<V: Ord> MinTerm<V> {
    fn index(&self, variable_map: &BTreeMap<&V, u32>) -> u32 {
        let mut result = 0u32;
        for var in self {
            let id = var.id();
            let idx = variable_map[id];
            if let Variable::Positive(_) = var {
                result |= 1 << idx;
            }
        }
        result
    }
}

impl<V: Ord> FromIterator<MinTerm<V>> for SOP<V> {
    fn from_iter<T: IntoIterator<Item = MinTerm<V>>>(iter: T) -> Self {
        let minterms = iter.into_iter().collect();
        Self { minterms }
    }
}

impl<T> BitOr for SOP<T>
where
    T: Ord + Clone,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut products = self.minterms;
        products.extend(rhs.minterms);
        SOP { minterms: products }
    }
}

impl<V> BitAnd for SOP<V>
where
    V: Ord + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let SOP { minterms: this } = self;
        let SOP { minterms: other } = rhs;
        let products = this
            .into_iter()
            .flat_map(|lhs_prod| {
                other
                    .clone()
                    .into_iter()
                    .map(move |rhs_prod| lhs_prod.clone() & rhs_prod)
            })
            .collect();
        SOP { minterms: products }
    }
}

impl<T> Display for SOP<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let SOP { minterms: products } = self;
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

    use super::{SOP, Variable};

    impl proptest::arbitrary::Arbitrary for Variable<u32> {
        type Parameters = (u32, bool);
        type Strategy = Just<Self>;

        fn arbitrary_with(args: Self::Parameters) -> Self::Strategy {
            let (id, negative) = args;
            if negative {
                Just(Variable::Negative(id))
            } else {
                Just(Variable::Positive(id))
            }
        }
    }

    fn evaluate(cond: SOP<u32>, value_map: &HashMap<u32, bool>) -> bool {
        cond.minterms
            .into_iter()
            .map(|product| product.into_iter().all(|it| value_map[it.id()]))
            .reduce(|lhs, rhs| lhs || rhs)
            .unwrap_or_default()
    }

    fn generate_pred_values(cond: &SOP<u32>) -> HashMap<u32, bool> {
        cond.minterms
            .iter()
            .flat_map(|it| it.0.iter())
            .map(Variable::id)
            .copied()
            .dedup()
            .map(|it| (it, rand::random()))
            .collect()
    }

    fn arb_test_cond() -> impl Strategy<Value = SOP<u32>> {
        btree_set(
            btree_set(any::<Variable<u32>>(), 1..20).prop_map(MinTerm),
            1..10,
        )
        .prop_map(|products| SOP { minterms: products })
    }

    proptest! {
        #[test]
        fn simplify(path_condition in arb_test_cond()) {
            let pred_values = generate_pred_values(&path_condition);
            let mut simplified = path_condition.clone();
            simplified.simplify();
            assert_eq!(
                evaluate(dbg!(path_condition), dbg!(&pred_values)),
                evaluate(dbg!(simplified), &pred_values),
            );
        }

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
