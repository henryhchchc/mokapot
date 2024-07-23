//! Path constraint analysis.
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Display,
};

use itertools::Itertools;

use crate::{
    analysis::fixed_point,
    ir::{self, control_flow::ControlTransfer, ControlFlowGraph, Operand},
    jvm::{code::ProgramCounter, ConstantValue},
};

/// Path condition in disjunctive normal form.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub struct PathCondition<T> {
    /// The set of conjunctive clauses.
    /// An empty set of conjunctive clauses represents a tautology.
    pub products: BTreeSet<BTreeSet<T>>,
}

impl<T> PathCondition<T> {
    fn simplify(&mut self)
    where
        T: Ord + Clone + std::ops::Not<Output = T>,
    {
        // We need a loop here since a simplification step may enable further simplifications.
        loop {
            let mut any_removal = false;
            // Remove contridictory products.
            self.products.retain(|product| {
                // If a product contains a condition and its negation, the product is contridictory.
                let shoule_remove = product.iter().any(|it| product.contains(&!it.clone()));
                any_removal |= shoule_remove;
                !shoule_remove
            });

            // Remove redundant products.
            let to_remove: BTreeSet<_> = self
                .products
                .iter()
                .filter(|product| {
                    // If a product is a super set of another product, the product is redundant.
                    self.products.iter().any(|another_product| {
                        *product != another_product && product.is_superset(another_product)
                    })
                })
                .cloned()
                .collect();
            any_removal |= !to_remove.is_empty();
            self.products.retain(|product| !to_remove.contains(product));

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

            if !any_removal {
                break;
            }
        }
    }
}

impl<T> Default for PathCondition<T> {
    fn default() -> Self {
        Self {
            products: BTreeSet::default(),
        }
    }
}

impl<T> std::ops::BitOr for PathCondition<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        let PathCondition { products: lhs } = self;
        let PathCondition { products: rhs } = rhs;
        let mut result = if lhs.is_empty() || rhs.is_empty() {
            PathCondition::default()
        } else {
            let products: BTreeSet<_> = lhs.into_iter().chain(rhs).collect();
            PathCondition { products }
        };
        result.simplify();
        result
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
        if this.is_empty() {
            PathCondition { products: other }
        } else if other.is_empty() {
            PathCondition { products: this }
        } else {
            let products = this
                .into_iter()
                .flat_map(|lhs_con| {
                    other.clone().into_iter().map(move |rhs_con| {
                        lhs_con.clone().into_iter().chain(rhs_con.clone()).collect()
                    })
                })
                .collect();
            let mut result = PathCondition { products };
            result.simplify();
            result
        }
    }
}

impl<T> std::ops::Not for PathCondition<T>
where
    T: Ord + Clone + std::ops::Not<Output = T>,
{
    type Output = Self;

    fn not(self) -> Self::Output {
        let PathCondition { products: clauses } = self;
        let mut result = clauses
            .into_iter()
            .map(|conj| PathCondition {
                products: conj.into_iter().map(|it| BTreeSet::from([!it])).collect(),
            })
            .reduce(|lhs, rhs| lhs | rhs)
            .unwrap_or_default();
        result.simplify();
        result
    }
}

impl<T> Display for PathCondition<T>
where
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let PathCondition { products: clauses } = self;
        for (i, conj) in clauses.iter().enumerate() {
            if i > 0 {
                write!(f, " || ")?;
            }
            if conj.len() > 1 {
                write!(f, "(")?;
            }
            for (j, cond) in conj.iter().enumerate() {
                if j > 0 {
                    write!(f, " && ")?;
                }
                write!(f, "{cond}")?;
            }
            if conj.len() > 1 {
                write!(f, ")")?;
            }
        }

        Ok(())
    }
}

/// A condition.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord)]
pub enum Predicate<V> {
    /// The left-hand side is equal to the right-hand side.
    Equal(V, V),
    /// The left-hand side is not equal to the right-hand side.
    NotEqual(V, V),
    /// The left-hand side is less than the right-hand side.
    LessThan(V, V),
    /// The left-hand side is less than or equal to the right-hand side.
    LessThanOrEqual(V, V),
    /// The value is null.
    IsNull(V),
    /// The value is not null.
    IsNotNull(V),
}

impl<V> std::ops::Not for Predicate<V> {
    type Output = Self;

    fn not(self) -> Self::Output {
        #[allow(clippy::enum_glob_use)]
        use Predicate::*;
        match self {
            Equal(lhs, rhs) => NotEqual(lhs, rhs),
            NotEqual(lhs, rhs) => Equal(lhs, rhs),
            LessThan(lhs, rhs) => LessThanOrEqual(rhs, lhs),
            LessThanOrEqual(lhs, rhs) => LessThan(rhs, lhs),
            IsNull(value) => IsNotNull(value),
            IsNotNull(value) => IsNull(value),
        }
    }
}

impl<V> Display for Predicate<V>
where
    V: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Equal(lhs, rhs) => write!(f, "{lhs} == {rhs}"),
            Self::NotEqual(lhs, rhs) => write!(f, "{lhs} != {rhs}"),
            Self::LessThan(lhs, rhs) => write!(f, "{lhs} < {rhs}"),
            Self::LessThanOrEqual(lhs, rhs) => write!(f, "{lhs} <= {rhs}"),
            Self::IsNull(value) => write!(f, "{value} == null"),
            Self::IsNotNull(value) => write!(f, "{value} != null"),
        }
    }
}

impl<C: Ord> From<Predicate<C>> for PathCondition<Predicate<C>> {
    fn from(value: Predicate<C>) -> Self {
        PathCondition {
            products: BTreeSet::from([BTreeSet::from([value])]),
        }
    }
}

impl From<ir::expression::Condition> for PathCondition<Predicate<Value>> {
    fn from(value: ir::expression::Condition) -> Self {
        #[allow(clippy::enum_glob_use)]
        use ir::expression::Condition::*;

        let zero = ConstantValue::Integer(0).into();
        let cond = match value {
            IsZero(value) => Predicate::Equal(value.into(), zero),
            IsNonZero(value) => Predicate::NotEqual(value.into(), zero),
            IsPositive(value) => Predicate::LessThan(zero, value.into()),
            IsNegative(value) => Predicate::LessThan(value.into(), zero),
            IsNonPositive(value) => Predicate::LessThanOrEqual(value.into(), zero),
            IsNonNegative(value) => Predicate::LessThanOrEqual(zero, value.into()),
            Equal(lhs, rhs) => Predicate::Equal(lhs.into(), rhs.into()),
            NotEqual(lhs, rhs) => Predicate::NotEqual(lhs.into(), rhs.into()),
            LessThan(lhs, rhs) => Predicate::LessThan(lhs.into(), rhs.into()),
            LessThanOrEqual(lhs, rhs) => Predicate::LessThanOrEqual(lhs.into(), rhs.into()),
            GreaterThan(lhs, rhs) => Predicate::LessThan(rhs.into(), lhs.into()),
            GreaterThanOrEqual(lhs, rhs) => Predicate::LessThanOrEqual(rhs.into(), lhs.into()),
            IsNull(value) => Predicate::IsNull(value.into()),
            IsNotNull(value) => Predicate::IsNotNull(value.into()),
        };
        cond.into()
    }
}

/// A value.
#[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, derive_more::Display)]
pub enum Value {
    /// A variable.
    Variable(Operand),
    /// A constant value.
    Constant(ConstantValue),
}

impl From<ir::Operand> for Value {
    fn from(value: ir::Operand) -> Self {
        Self::Variable(value)
    }
}

impl From<ConstantValue> for Value {
    fn from(value: ConstantValue) -> Self {
        Self::Constant(value)
    }
}

/// An analyzer for path conditions.
#[derive(Debug)]
pub struct Analyzer<'a> {
    cfg: &'a ControlFlowGraph<(), ControlTransfer>,
}

impl<'a> Analyzer<'a> {
    /// Creates a new path condition analyzer.
    #[must_use]
    pub fn new(cfg: &'a ControlFlowGraph<(), ControlTransfer>) -> Self {
        Self { cfg }
    }
}

impl fixed_point::Analyzer for Analyzer<'_> {
    type Location = ProgramCounter;

    type Fact = PathCondition<Predicate<Value>>;

    type Err = ();

    type AffectedLocations = BTreeMap<Self::Location, Self::Fact>;

    fn entry_fact(&self) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(BTreeMap::from([(
            ProgramCounter::ZERO,
            Self::Fact::default(),
        )]))
    }

    fn analyze_location(
        &mut self,
        location: &Self::Location,
        fact: &Self::Fact,
    ) -> Result<Self::AffectedLocations, Self::Err> {
        Ok(self
            .cfg
            .edges_from(*location)
            .into_iter()
            .flatten()
            .map(|(_, dst, trx)| match trx {
                ControlTransfer::Conditional(cond) => (dst, cond.clone() & fact.clone()),
                _ => (dst, fact.clone()),
            })
            .collect())
    }

    fn merge_facts(
        &self,
        current_fact: &Self::Fact,
        incoming_fact: Self::Fact,
    ) -> Result<Self::Fact, Self::Err> {
        let mut result = current_fact.clone() | incoming_fact;
        result.simplify();
        Ok(result)
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use itertools::Itertools;
    use proptest::prelude::*;

    use super::PathCondition;

    #[derive(Debug, PartialEq, Eq, Clone, PartialOrd, Ord, proptest_derive::Arbitrary)]
    struct TestPredicate(pub String, pub bool);

    impl std::ops::Not for TestPredicate {
        type Output = Self;

        fn not(self) -> Self::Output {
            Self(self.0, !self.1)
        }
    }

    fn evaluate(cond: PathCondition<TestPredicate>, value_map: &HashMap<String, bool>) -> bool {
        cond.products
            .into_iter()
            .map(|product| product.into_iter().all(|it| (value_map[&it.0] == it.1)))
            .fold(true, |acc, it| acc || it)
    }

    proptest! {
        #[test]
        fn simplify(
            products in prop::collection::btree_set(
                prop::collection::btree_set(any::<TestPredicate>(), 1..10),
                1..10
            )
        ) {
            let mut rng = rand::thread_rng();
            let pred_values = products
                .iter()
                .flatten()
                .map(|it|&it.0)
                .dedup()
                .map(|it| (it.clone(), rng.gen::<bool>()))
                .collect::<HashMap<_,_>>();
            let path_condition = super::PathCondition { products };
            let mut simplified = path_condition.clone();
            simplified.simplify();
            assert_eq!(
                evaluate(dbg!(path_condition), dbg!(&pred_values)),
                evaluate(dbg!(simplified), &pred_values),
            );
        }
    }
}
