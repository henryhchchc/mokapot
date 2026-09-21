use std::{
    collections::{HashMap, HashSet},
    hash::Hash,
};

use proptest::{collection::hash_set, prelude::*};

use super::{
    BooleanVariable, BranchGuard, PathCondition, PathConditionTerm, SolvingBudget, cover::Cover,
};

impl<P> PathConditionTerm<'_, P> {
    /// Iterates over this term's literals.
    ///
    /// The iteration order is unspecified.
    pub fn literals(&self) -> impl Iterator<Item = BooleanVariable<&P>> {
        self.0.literals()
    }
}

impl<P> PathCondition<P> {
    fn from_branch_guards(branch_guards: impl IntoIterator<Item = BranchGuard<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(Cover::from_branch_guards(branch_guards))
    }
}

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
    cond.disjuncts()
        .map(|term| {
            term.literals().all(|it| match it {
                BooleanVariable::Positive(id) => value_map[id],
                BooleanVariable::Negative(id) => !value_map[id],
            })
        })
        .reduce(|lhs, rhs| lhs || rhs)
        .unwrap_or_default()
}

#[test]
fn exposes_dnf_terms_and_guard_literals() {
    let guard = BranchGuard::from_iter([
        BooleanVariable::Positive(1_u32),
        BooleanVariable::Negative(2),
    ]);
    assert_eq!(
        guard.literals().collect::<HashSet<_>>(),
        HashSet::from([BooleanVariable::Positive(&1), BooleanVariable::Negative(&2),])
    );

    let condition = PathCondition::one() & guard;
    let terms = condition.disjuncts().collect::<Vec<_>>();
    assert_eq!(terms.len(), 1);
    assert!(!terms[0].is_tautology());
    assert_eq!(
        terms[0].literals().collect::<HashSet<_>>(),
        HashSet::from([BooleanVariable::Positive(&1), BooleanVariable::Negative(&2),])
    );

    let tautology_condition = PathCondition::<u32>::one();
    let tautology = tautology_condition.disjuncts().collect::<Vec<_>>();
    assert_eq!(tautology.len(), 1);
    assert!(tautology[0].is_tautology());
    assert!(PathCondition::<u32>::zero().disjuncts().next().is_none());
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
}

mod explicit_reduction {
    use super::*;

    #[test]
    fn reduce_eliminates_complementary_terms_explicitly() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let structural =
            (PathCondition::of(a.clone()) & b.clone()) | (PathCondition::of(a.clone()) & !b);

        let reduced = structural
            .clone()
            .reduce_with_budget(SolvingBudget::default());

        assert_ne!(structural, PathCondition::of(a.clone()));
        assert_eq!(reduced, PathCondition::of(a));
    }
}
