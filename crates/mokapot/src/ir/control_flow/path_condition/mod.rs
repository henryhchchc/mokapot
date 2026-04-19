//! Path condition analysis.

use std::{
    collections::HashMap,
    collections::HashSet,
    fmt::Display,
    hash::{Hash, Hasher},
    ops::{BitAnd, BitOr},
};

use crate::{
    ir::{ControlFlowGraph, control_flow::ControlTransfer, expression::Condition},
    jvm::code::ProgramCounter,
};
use itertools::Itertools;

mod analyzer;
mod branch_guard;
mod budget;
mod cover;
mod cube;
mod literal;
mod minimizer;
mod predicate;

use cover::Cover;

pub use branch_guard::BranchGuard;
pub use budget::PathConditionBudget;
pub use literal::BooleanVariable;
pub use predicate::Value;

pub(super) fn analyze<N>(
    cfg: &ControlFlowGraph<N, ControlTransfer>,
    budget: PathConditionBudget,
) -> HashMap<ProgramCounter, PathCondition<&Condition<Value>>> {
    analyzer::analyze(cfg, budget)
}

/// A path condition stored in disjunctive normal form.
#[derive(Debug, Clone)]
pub struct PathCondition<P> {
    cover: Cover<P>,
}

impl<P> PartialEq for PathCondition<P>
where
    P: Hash + Eq,
{
    fn eq(&self, other: &Self) -> bool {
        self.cover == other.cover
    }
}

impl<P> Eq for PathCondition<P> where P: Hash + Eq {}

impl<P> Hash for PathCondition<P>
where
    P: Hash + Eq,
{
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.cover.hash(state);
    }
}

impl<P> PathCondition<P> {
    const fn with_cover(cover: Cover<P>) -> Self {
        Self { cover }
    }

    /// Creates the tautological condition `⊤`.
    #[must_use]
    pub fn one() -> Self
    where
        P: Hash + Eq,
    {
        Self::with_cover(Cover::one())
    }

    /// Creates the contradictory condition `⊥`.
    #[must_use]
    pub fn zero() -> Self {
        Self::with_cover(Cover::zero())
    }

    /// Creates a path condition from a single literal.
    #[must_use]
    pub fn of(predicate: BooleanVariable<P>) -> Self
    where
        P: Hash + Eq,
    {
        Self::with_cover(Cover::of_literal(predicate))
    }

    /// Returns the predicates referenced by this condition.
    #[must_use]
    pub fn predicates(&self) -> HashSet<&P>
    where
        P: Hash + Eq,
    {
        self.cover.predicates().collect()
    }

    /// Returns whether this condition is `⊥`.
    #[must_use]
    pub fn is_contradiction(&self) -> bool {
        self.cover.is_contradiction()
    }

    /// Reduces this condition with the given minimization budget.
    ///
    /// This is an explicit structural optimization step. Raw boolean
    /// composition on [`PathCondition`] does not perform semantic
    /// minimization implicitly.
    #[must_use]
    pub fn reduce(self, budget: PathConditionBudget) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(self.cover.reduce(budget))
    }

    #[cfg(test)]
    fn from_branch_guards(branch_guards: impl IntoIterator<Item = BranchGuard<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(Cover::from_branch_guards(branch_guards))
    }

    #[cfg(test)]
    fn cubes(&self) -> impl Iterator<Item = &cube::Cube<P>> {
        self.cover.cubes()
    }
}

impl<P> BitOr for PathCondition<P>
where
    P: Hash + Eq,
{
    type Output = Self;

    fn bitor(self, rhs: Self) -> Self::Output {
        Self::with_cover(self.cover.disjoin(rhs.cover))
    }
}

impl<P> BitAnd<BooleanVariable<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BooleanVariable<P>) -> Self::Output {
        Self::with_cover(self.cover.conjoin_literal(&rhs))
    }
}

impl<P> BitAnd<BranchGuard<P>> for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: BranchGuard<P>) -> Self::Output {
        Self::with_cover(self.cover.conjoin_branch_guard(rhs))
    }
}

impl<P> BitAnd for PathCondition<P>
where
    P: Hash + Eq + Clone,
{
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        Self::with_cover(self.cover.conjoin(&rhs.cover))
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
            let cubes = self
                .cover
                .cubes()
                .map(ToString::to_string)
                .sorted()
                .collect::<Vec<_>>();
            write!(f, "{}", cubes.iter().format(" || "))
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::{collection::hash_set, prelude::*};

    use super::{BooleanVariable, BranchGuard, PathCondition, PathConditionBudget};

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

    fn evaluate(cond: &PathCondition<u32>, value_map: &HashMap<u32, bool>) -> bool {
        cond.cubes()
            .map(|cube| {
                cube.literals().all(|it| match it {
                    BooleanVariable::Positive(id) => value_map[id],
                    BooleanVariable::Negative(id) => !value_map[id],
                })
            })
            .reduce(|lhs, rhs| lhs || rhs)
            .unwrap_or_default()
    }

    fn generate_pred_values(cond: &PathCondition<u32>) -> HashMap<u32, bool> {
        cond.predicates()
            .into_iter()
            .copied()
            .map(|predicate| (predicate, rand::random()))
            .collect()
    }

    fn arb_test_cond() -> impl Strategy<Value = PathCondition<u32>> {
        hash_set(
            hash_set(any::<BooleanVariable<u32>>(), 1..26).prop_map(BranchGuard),
            1..26,
        )
        .prop_map(PathCondition::from_branch_guards)
    }

    fn conjunction(literals: impl IntoIterator<Item = BooleanVariable<u32>>) -> PathCondition<u32> {
        literals
            .into_iter()
            .fold(PathCondition::one(), |condition, literal| {
                condition & literal
            })
    }

    mod raw_structure {
        use super::*;

        proptest! {
            #[test]
            fn conjunction_matches_boolean_semantics(
                lhs in arb_test_cond(),
                rhs in arb_test_cond()
            ) {
                let mut pred_values = generate_pred_values(&lhs);
                pred_values.extend(generate_pred_values(&rhs));
                let lhs_eval = evaluate(&lhs, &pred_values);
                let rhs_eval = evaluate(&rhs, &pred_values);
                let conjunction = lhs.clone() & rhs.clone();
                let conjunction_eval = evaluate(&conjunction, &pred_values);
                assert_eq!(lhs_eval && rhs_eval, conjunction_eval);
            }

            #[test]
            fn disjunction_matches_boolean_semantics(
                lhs in arb_test_cond(),
                rhs in arb_test_cond()
            ) {
                let mut pred_values = generate_pred_values(&lhs);
                pred_values.extend(generate_pred_values(&rhs));
                let lhs_eval = evaluate(&lhs, &pred_values);
                let rhs_eval = evaluate(&rhs, &pred_values);
                let disjunction = lhs.clone() | rhs.clone();
                let disjunction_eval = evaluate(&disjunction, &pred_values);
                assert_eq!(lhs_eval || rhs_eval, disjunction_eval);
            }
        }

        #[test]
        fn conjunction_eliminates_direct_contradictions() {
            let lhs = PathCondition::one() & BooleanVariable::Positive(1_u32);
            let rhs = lhs & BooleanVariable::Negative(1_u32);
            assert_eq!(rhs, PathCondition::zero());
        }

        #[test]
        fn disjunction_preserves_more_specific_terms_structurally() {
            let a = BooleanVariable::Positive(1_u32);
            let b = BooleanVariable::Positive(2_u32);
            let lhs = PathCondition::of(a.clone());
            let rhs = PathCondition::of(a.clone()) & b;
            assert_ne!(lhs.clone() | rhs, lhs);
        }

        #[test]
        fn disjunction_preserves_complementary_terms_structurally() {
            let a = BooleanVariable::Positive(1_u32);
            let b = BooleanVariable::Positive(2_u32);
            let lhs = PathCondition::of(a.clone()) & b.clone();
            let rhs = PathCondition::of(a.clone()) & !b;
            assert_ne!(lhs | rhs, PathCondition::of(a));
        }

        #[test]
        fn structurally_distinct_equivalent_forms_are_not_equal_without_reduction() {
            let a = BooleanVariable::Positive(1_u32);
            let b = BooleanVariable::Positive(2_u32);
            let specific = PathCondition::of(a.clone()) & b.clone();
            let general = PathCondition::of(a.clone()) & b.clone() | (PathCondition::of(a) & !b);
            assert_ne!(specific, general);
        }

        #[test]
        fn disjunction_order_is_irrelevant_even_for_complex_covers() {
            let a = BooleanVariable::Positive(1_u32);
            let b = BooleanVariable::Positive(2_u32);
            let c = BooleanVariable::Positive(3_u32);

            let branch_guards = [
                conjunction([!a.clone(), !b.clone(), !c.clone()]),
                conjunction([!a.clone(), b.clone(), !c.clone()]),
                conjunction([!a.clone(), b.clone(), c.clone()]),
                conjunction([a.clone(), !b.clone(), !c.clone()]),
                conjunction([a.clone(), !b.clone(), c.clone()]),
            ];

            let lhs = branch_guards
                .iter()
                .cloned()
                .fold(PathCondition::zero(), |condition, branch_guard| {
                    condition | branch_guard
                });
            let rhs = branch_guards
                .iter()
                .rev()
                .cloned()
                .fold(PathCondition::zero(), |condition, branch_guard| {
                    condition | branch_guard
                });

            assert_eq!(lhs, rhs);
        }

        #[test]
        fn display_sorts_literals_within_a_cube() {
            let lhs = conjunction([
                BooleanVariable::Positive(2_u32),
                BooleanVariable::Negative(1_u32),
            ]);
            let rhs = conjunction([
                BooleanVariable::Negative(1_u32),
                BooleanVariable::Positive(2_u32),
            ]);
            assert_eq!(lhs.to_string(), rhs.to_string());
        }

        #[test]
        fn display_sorts_cubes_within_a_condition() {
            let lhs = conjunction([BooleanVariable::Positive(2_u32)])
                | conjunction([BooleanVariable::Positive(1_u32)]);
            let rhs = conjunction([BooleanVariable::Positive(1_u32)])
                | conjunction([BooleanVariable::Positive(2_u32)]);
            assert_eq!(lhs.to_string(), rhs.to_string());
        }
    }

    mod explicit_reduction {
        use super::*;

        #[test]
        fn reduce_eliminates_complementary_terms_explicitly() {
            let a = BooleanVariable::Positive(1_u32);
            let b = BooleanVariable::Positive(2_u32);
            let structural =
                (PathCondition::of(a.clone()) & b.clone()) | (PathCondition::of(a.clone()) & !b);

            let reduced = structural.clone().reduce(PathConditionBudget::default());

            assert_ne!(structural, PathCondition::of(a.clone()));
            assert_eq!(reduced, PathCondition::of(a));
        }
    }
}
