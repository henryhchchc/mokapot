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

    /// Returns an iterator over the variables in the minterm.
    pub fn iter(&self) -> impl Iterator<Item = &Variable<P>> {
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
    ///
    /// # Panics
    /// - When there are more than 26 variables (due to `quine_mccluskey` limitation)
    pub fn simplify(&mut self)
    where
        P: Ord + Clone,
    {
        use quine_mccluskey as qmc;
        if self == &Self::one() || self == &Self::zero() {
            return;
        }
        let variable_ids = self.variable_ids();
        let idx_count = variable_ids.len();
        if variable_ids.len() > qmc::DEFAULT_VARIABLES.len() {
            todo!("Too many variables")
        }
        let var_idx: BTreeMap<_, _> = variable_ids
            .iter()
            .enumerate()
            .map(|(idx, var)| (*var, u32::try_from(idx).unwrap()))
            .collect();
        let var_map: BTreeMap<_, _> = qmc::DEFAULT_VARIABLES
            .into_iter()
            .zip(variable_ids)
            .collect();
        let minterms: Vec<_> = self.minterms.iter().map(|it| it.index(&var_idx)).collect();
        let solutions = qmc::minimize_minterms(
            &qmc::DEFAULT_VARIABLES[..idx_count],
            &minterms,
            &[],
            true,
            None,
        )
        .expect("There should be a result as no timeout is set.");
        let shortest_solution = solutions
            .into_iter()
            .min_by_key(|sol| match sol {
                quine_mccluskey::Solution::One | quine_mccluskey::Solution::Zero => 0,
                quine_mccluskey::Solution::SOP(vec) => vec
                    .iter()
                    .flatten()
                    .map(|it| it.name.as_str())
                    .dedup()
                    .count(),
                quine_mccluskey::Solution::POS(_) => unreachable!(),
            })
            .expect("There should be at least one solution");
        let new_sop = match shortest_solution {
            qmc::Solution::SOP(sop_solution) => sop_solution
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
                .collect(),
            qmc::Solution::One => SOP::one(),
            qmc::Solution::Zero => SOP::zero(),
            qmc::Solution::POS(_) => unreachable!("We are using `minimize_minterms`"),
        };
        *self = new_sop;
    }

    fn variable_ids(&self) -> BTreeSet<&P>
    where
        P: Ord,
    {
        self.minterms.iter().flatten().map(Variable::id).collect()
    }
}

impl<V: Ord> MinTerm<V> {
    fn index(&self, variable_map: &BTreeMap<&V, u32>) -> u32 {
        debug_assert!(!variable_map.is_empty());
        let mut result = 0u32;
        let max_idx = u32::try_from(variable_map.len() - 1).expect("u32 cannot hold max idx");
        for var in self {
            let id = var.id();
            let idx = variable_map[id];
            if let Variable::Positive(_) = var {
                result |= 1 << (max_idx - idx);
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
            .map(|minterm| {
                minterm.into_iter().all(|ref it| match it {
                    Variable::Positive(id) => value_map[id],
                    Variable::Negative(id) => !value_map[id],
                })
            })
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
            btree_set(any::<Variable<u32>>(), 1..26).prop_map(MinTerm),
            1..26,
        )
        .prop_map(|minterms| SOP { minterms })
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
            let mut conjunction = lhs.clone() & rhs.clone();
            conjunction.simplify();
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
            let mut disjunction = lhs.clone() | rhs.clone();
            disjunction.simplify();
            let disjunction_eval = evaluate(disjunction.clone(), &pred_values);
            assert_eq!(lhs_eval || rhs_eval, disjunction_eval);
        }
    }
}
