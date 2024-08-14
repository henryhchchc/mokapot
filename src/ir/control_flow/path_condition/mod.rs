//! Path constraint analysis.
use std::{
    collections::BTreeSet,
    fmt::Display,
    ops::{Deref, DerefMut},
};

use itertools::Itertools;

mod analyzer;

pub use analyzer::*;

/// Path condition in disjunctive normal form.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PathCondition<P> {
    /// The clauses in the disjunctive normal form.
    /// An empry set represents a contradiction.
    /// An singleton of an empty set represents a tautology.
    products: BTreeSet<Conjunction<P>>,
}

/// A conjunction of predicates.
#[derive(Debug, PartialEq, Eq, Clone)]
pub struct Conjunction<P>(BTreeSet<P>);

impl<P: Ord> FromIterator<P> for Conjunction<P> {
    fn from_iter<T: IntoIterator<Item = P>>(iter: T) -> Self {
        Self(BTreeSet::from_iter(iter))
    }
}

impl<P> Conjunction<P> {
    /// Creates a conjunction of predicates.
    #[must_use]
    pub fn from_predicates(predicates: impl IntoIterator<Item = P>) -> Self
    where
        P: Ord,
    {
        Self(BTreeSet::from_iter(predicates))
    }

    /// Creates a tautology.
    #[must_use]
    pub fn tautology() -> Self {
        Self(BTreeSet::default())
    }
}

impl<P: Display> Display for Conjunction<P> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.is_empty() {
            write!(f, "⊤")
        } else {
            write!(f, "{}", self.0.iter().format(" && "))
        }
    }
}

impl<P> Deref for Conjunction<P> {
    type Target = BTreeSet<P>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<P> DerefMut for Conjunction<P> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<P> IntoIterator for Conjunction<P> {
    type Item = P;
    type IntoIter = std::collections::btree_set::IntoIter<P>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<P> PartialOrd for Conjunction<P>
where
    P: PartialOrd,
{
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.len() == other.len() {
            self.0.partial_cmp(&other.0)
        } else {
            self.len().partial_cmp(&other.len())
        }
    }
}

impl<P> Ord for Conjunction<P>
where
    P: Ord,
{
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        if self.len() == other.len() {
            self.0.cmp(&other.0)
        } else {
            self.len().cmp(&other.len())
        }
    }
}

impl<P> Deref for PathCondition<P> {
    type Target = BTreeSet<Conjunction<P>>;

    fn deref(&self) -> &Self::Target {
        &self.products
    }
}

impl<P> PathCondition<P> {
    /// Creates a tautology.
    #[must_use]
    pub fn tautology() -> Self
    where
        P: Ord,
    {
        let products = BTreeSet::from([Conjunction::tautology()]);
        Self { products }
    }

    /// Creates a contradiction.
    #[must_use]
    pub fn contradiction() -> Self
    where
        P: Ord,
    {
        let products = BTreeSet::default();
        Self { products }
    }

    /// Creates a conjuction of predicates.
    #[must_use]
    pub fn conjuction_of(predicates: impl IntoIterator<Item = P>) -> Self
    where
        P: Ord,
    {
        let product = Conjunction::from_predicates(predicates);
        let products = BTreeSet::from([product]);
        Self { products }
    }

    /// Simplifies the path condition.
    #[stability::unstable(feature = "path-condition", issue = "10")]
    // FIXME: The current implementation is buggy. See Issue #10.
    pub fn simplify(&mut self)
    where
        P: Ord + Clone + std::ops::Not<Output = P>,
    {
        loop {
            let mut any_removal = false;
            // Apply absorption laws.
            // i.e. Aa + A!ab = Aa + Ab
            let pairs_of_products: Vec<_> = self
                .products
                .iter()
                .flat_map(|lhs| self.products.iter().map(move |rhs| (lhs, rhs)))
                .collect();

            let new_products: BTreeSet<_> = pairs_of_products
                .into_iter()
                .filter_map(|(lhs, rhs)| {
                    if let Some((single,)) = lhs.difference(rhs).collect_tuple() {
                        let mut rhs_diff = rhs.difference(lhs).collect::<BTreeSet<_>>();
                        if rhs_diff.contains(&!single.clone()) {
                            rhs_diff.remove(&!single.clone());
                            let factor: BTreeSet<_> = lhs.intersection(rhs).collect();
                            let new_rhs = rhs_diff.union(&factor).map(|it| (*it).clone()).collect();
                            return Some(new_rhs);
                        }
                    }
                    None
                })
                .collect();
            // Adding simplified new products will lead to removal in the next iteration.
            any_removal |= !new_products.is_empty();
            self.products.extend(new_products);

            // Remove redundant products.
            let to_remove: BTreeSet<_> = self
                .products
                .iter()
                .filter(|product| {
                    // If a product is a super set of another product, the product is redundant.
                    self.products.iter().any(|another_product| {
                        product.len() > another_product.len()
                            && product.is_superset(another_product)
                    })
                })
                .cloned()
                .collect();
            any_removal |= !to_remove.is_empty();
            self.products.retain(|product| !to_remove.contains(product));

            if !any_removal {
                break;
            }
        }
    }
}

impl<T> std::ops::BitOr for PathCondition<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let mut products = self.products;
        products.extend(rhs.products);
        PathCondition { products }
    }
}

impl<T> std::ops::BitAnd for PathCondition<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        let PathCondition { products: this } = self;
        let PathCondition { products: other } = rhs;
        let products = this
            .into_iter()
            .flat_map(|lhs_prod| {
                other.clone().into_iter().map(move |rhs_prod| {
                    let mut prod = lhs_prod.clone();
                    prod.extend(rhs_prod);
                    prod
                })
            })
            .filter(|product| !product.iter().any(|it| product.contains(&!it.clone())))
            .collect();
        PathCondition { products }
    }
}

impl<T> Display for PathCondition<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PathCondition { products } = self;
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

    use crate::ir::control_flow::path_condition::Conjunction;

    use super::PathCondition;

    #[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, proptest_derive::Arbitrary)]
    struct TestPredicate(pub u32, pub bool);

    impl std::ops::Not for TestPredicate {
        type Output = Self;

        fn not(self) -> Self::Output {
            Self(self.0, !self.1)
        }
    }

    fn evaluate(cond: PathCondition<TestPredicate>, value_map: &HashMap<u32, bool>) -> bool {
        cond.products
            .into_iter()
            .map(|product| product.into_iter().all(|it| value_map[&it.0] == it.1))
            .reduce(|lhs, rhs| lhs || rhs)
            .unwrap_or_default()
    }

    fn generate_pred_values(cond: &PathCondition<TestPredicate>) -> HashMap<u32, bool> {
        let mut rng = rand::thread_rng();
        cond.iter()
            .flat_map(|it| it.iter())
            .map(|it| &it.0)
            .dedup()
            .map(|it| (*it, rng.gen::<bool>()))
            .collect()
    }

    fn arb_test_cond() -> impl Strategy<Value = PathCondition<TestPredicate>> {
        btree_set(
            btree_set(any::<TestPredicate>(), 1..10).prop_map(Conjunction),
            1..10,
        )
        .prop_map(|products| PathCondition { products })
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
            let conjuction = lhs.clone() & rhs.clone();
            let conjuction_eval = evaluate(conjuction.clone(), &pred_values);
            assert_eq!(lhs_eval && rhs_eval, conjuction_eval);
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
