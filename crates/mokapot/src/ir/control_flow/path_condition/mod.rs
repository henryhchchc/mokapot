//! Path constraint analysis.
//!
//! This module implements path condition analysis using disjunctive normal form (DNF).
//! A path condition represents a boolean formula that must be satisfied for a path to be taken.

mod analyzer;
mod cover;
mod cube;
mod literal;
mod minimizer;
mod minterm;
mod predicate;

pub use analyzer::Analyzer;
pub use cover::PathCondition;
pub use literal::BooleanVariable;
pub use minterm::MinTerm;
pub use predicate::Value;

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use proptest::{collection::hash_set, prelude::*};

    use super::{BooleanVariable, MinTerm, PathCondition};

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
            hash_set(any::<BooleanVariable<u32>>(), 1..26).prop_map(MinTerm),
            1..26,
        )
        .prop_map(PathCondition::from_minterms)
    }

    proptest! {
        #[test]
        fn and(
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
        fn or(
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
    fn or_absorbs_more_specific_terms() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let lhs = PathCondition::of(a.clone());
        let rhs = PathCondition::of(a.clone()) & b;
        assert_eq!(lhs.clone() | rhs, lhs);
    }

    #[test]
    fn and_eliminates_direct_contradictions() {
        let lhs = PathCondition::one() & BooleanVariable::Positive(1_u32);
        let rhs = lhs & BooleanVariable::Negative(1_u32);
        assert_eq!(rhs, PathCondition::zero());
    }
}
