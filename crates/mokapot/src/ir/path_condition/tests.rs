use std::{collections::HashMap, hash::Hash};

use proptest::{collection::hash_set, prelude::*};

use super::{BranchGuard, PathCondition, SolvingBudget, cover::Cover};
use crate::ir::expression::BooleanVariable;

impl<P> PathCondition<P> {
    fn from_branch_guards(branch_guards: impl IntoIterator<Item = BranchGuard<P>>) -> Self
    where
        P: Hash + Eq + Clone,
    {
        Self::with_cover(Cover::from_branch_guards(branch_guards))
    }
}

/// Evaluates a condition under `value_map` by unfolding its disjunctive normal form.
fn evaluate(cond: &PathCondition<u32>, value_map: &HashMap<u32, bool>) -> bool {
    cond.cover.cubes().any(|cube| {
        cube.literals().all(|it| match it {
            BooleanVariable::Positive(id) => value_map[id],
            BooleanVariable::Negative(id) => !value_map[id],
        })
    })
}

/// The predicate ids every generated condition draws from.
const PREDICATE_IDS: u32 = 4;

/// A conjunction of one or more literals, the shape a conditional CFG edge carries.
fn arb_branch_guard() -> impl Strategy<Value = BranchGuard<u32>> {
    hash_set(
        (0..PREDICATE_IDS, any::<bool>()).prop_map(|(predicate, negative)| {
            if negative {
                BooleanVariable::Negative(predicate)
            } else {
                BooleanVariable::Positive(predicate)
            }
        }),
        1..=PREDICATE_IDS as usize,
    )
    .prop_map(BranchGuard::from_iter)
}

/// A condition with a structured cover: a disjunction of one or more branch guards.
fn arb_structured_condition() -> impl Strategy<Value = PathCondition<u32>> {
    hash_set(arb_branch_guard(), 1..=4).prop_map(PathCondition::from_branch_guards)
}

/// Any condition, including the tautology `⊤` and the contradiction `⊥`.
fn arb_condition() -> impl Strategy<Value = PathCondition<u32>> {
    prop_oneof![
        Just(PathCondition::<u32>::one()),
        Just(PathCondition::<u32>::zero()),
        arb_structured_condition(),
    ]
}

/// A total truth assignment over every predicate id a generated condition can reference.
fn arb_values() -> impl Strategy<Value = HashMap<u32, bool>> {
    prop::collection::vec(any::<bool>(), PREDICATE_IDS as usize)
        .prop_map(|values| (0..PREDICATE_IDS).zip(values).collect())
}

/// Budgets that exercise both the exact and the bounded heuristic reducer.
fn arb_budget() -> impl Strategy<Value = SolvingBudget> {
    (0..=8_usize, 0..=4_usize, 0..=64_usize).prop_map(
        |(on_set_size, heuristic_rounds, cover_checks)| SolvingBudget {
            on_set_size,
            heuristic_rounds,
            cover_checks,
        },
    )
}

mod raw_structure {
    use super::*;

    proptest! {
        #[test]
        fn disjunction_matches_boolean_semantics(
            lhs in arb_condition(),
            rhs in arb_condition(),
            values in arb_values(),
        ) {
            let disjunction = lhs.clone() | rhs.clone();
            prop_assert_eq!(
                evaluate(&lhs, &values) || evaluate(&rhs, &values),
                evaluate(&disjunction, &values),
            );
        }
    }

    #[test]
    fn conjunction_eliminates_direct_contradictions() {
        let guard = BranchGuard::from_iter([
            BooleanVariable::Positive(1_u32),
            BooleanVariable::Negative(1_u32),
        ]);
        let rhs = PathCondition::one() & guard;
        assert_eq!(rhs, PathCondition::zero());
    }
}

mod explicit_reduction {
    use super::*;

    #[test]
    fn reduce_eliminates_complementary_terms_explicitly() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let structural = PathCondition::from_branch_guards([
            BranchGuard::from_iter([a.clone(), b.clone()]),
            BranchGuard::from_iter([a.clone(), !b]),
        ]);
        let expected = PathCondition::one() & BranchGuard::of(a);

        let reduced = structural
            .clone()
            .reduce_with_budget(SolvingBudget::default());

        assert_ne!(structural, expected);
        assert_eq!(reduced, expected);
    }

    #[test]
    fn semantic_order_compares_meaning_not_form() {
        let a = BooleanVariable::Positive(1_u32);
        let b = BooleanVariable::Positive(2_u32);
        let structural = PathCondition::from_branch_guards([
            BranchGuard::from_iter([a.clone(), b.clone()]),
            BranchGuard::from_iter([a.clone(), !b.clone()]),
        ]);
        let expected = PathCondition::one() & BranchGuard::of(a);
        let different = PathCondition::one() & BranchGuard::of(b);

        assert_ne!(structural, expected);
        assert_eq!(
            structural.cover.partial_cmp(&expected.cover),
            Some(std::cmp::Ordering::Equal)
        );
        assert_ne!(
            structural.cover.partial_cmp(&different.cover),
            Some(std::cmp::Ordering::Equal)
        );
    }

    proptest! {
        /// Reduction keeps the meaning of a condition under every budget.
        #[test]
        fn reduction_preserves_meaning(
            condition in arb_condition(),
            values in arb_values(),
            budget in arb_budget(),
        ) {
            let reduced = condition.clone().reduce_with_budget(budget);
            prop_assert_eq!(
                evaluate(&condition, &values),
                evaluate(&reduced, &values),
                "reduction changed the meaning of {}",
                condition,
            );
        }
    }
}

mod analyzer {
    use std::collections::HashMap;

    use crate::{
        ir::{
            BranchGuard, ControlTransfer as Transfer, NumericalId,
            expression::{BooleanVariable, Expression, PathValue, Predicate},
            path_condition::PathCondition,
            test::prelude::*,
        },
        jvm::ConstantValue::{Integer, Null},
    };

    #[test]
    fn path_conditions_prune_contradictory_arms_at_block_locations() {
        let condition = Predicate::IsZero(ValueId::from_raw(0).into());
        let positive: BooleanVariable<Predicate> = condition.into();
        let negative = !positive.clone();
        let [b0, b1, b2, b3] = ids(0);
        let taken = Transfer::Conditional(BranchGuard::of(positive));
        let missed = Transfer::Conditional(BranchGuard::of(negative));
        let exit = void();

        let blocks = HashMap::from([
            code(b0, goto_with(b1, [], taken)),
            code(b1, branch(arm(b2, [], missed), edge(b3, []))),
            code(b2, exit.clone()),
            code(b3, exit),
        ]);
        let method = ir_method(b0, blocks);
        let conditions = PathCondition::analyze(&method);

        assert!(conditions.contains_key(&b0));
        assert!(conditions.contains_key(&b1));
        assert!(!conditions.contains_key(&b2));
        assert!(conditions.contains_key(&b3));
    }

    #[test]
    fn exceptional_outcomes_preserve_the_incoming_path_condition() {
        let condition = Predicate::IsZero(ValueId::from_raw(0).into());
        let positive: BooleanVariable<Predicate> = condition.into();
        let negative = !positive.clone();
        let [entry_id, tried, normal, handler, otherwise] = ids(0);
        let taken = Transfer::Conditional(BranchGuard::of(positive));
        let missed = Transfer::Conditional(BranchGuard::of(negative));
        let caught = Transfer::Exception(Some("java/lang/RuntimeException".parse().unwrap()));
        let exit = void();

        let selecting = branch(arm(tried, [], taken), arm(otherwise, [], missed));
        let exceptional = vec![arm(handler, [], caught), Successor::Unwind];
        let fallible = try_op(effect(Null), edge(normal, []), exceptional);
        let blocks = HashMap::from([
            code(entry_id, selecting),
            code(tried, fallible),
            code(normal, exit.clone()),
            code(handler, exit.clone()),
            code(otherwise, exit),
        ]);
        let method = ir_method(entry_id, blocks);
        let conditions = PathCondition::analyze(&method);

        assert_eq!(conditions[&tried], conditions[&normal]);
        assert_eq!(conditions[&tried], conditions[&handler]);
        assert_ne!(conditions[&tried], conditions[&otherwise]);
    }

    #[test]
    fn constant_definition_prunes_a_branch_in_a_later_block() {
        let [entry, branch_id, impossible, reachable] = ids(0);
        let [zero]: [ValueId; 1] = ids(0);
        let predicate = Predicate::IsZero(zero.into());
        let false_guard = Transfer::Conditional(BranchGuard::of(!BooleanVariable::from(predicate)));
        let blocks = HashMap::from([
            bb(
                entry,
                [],
                &[def(zero, Expression::Const(Integer(0)))],
                goto(branch_id, []),
            ),
            code(
                branch_id,
                branch(arm(impossible, [], false_guard), edge(reachable, [])),
            ),
            code(impossible, void()),
            code(reachable, void()),
        ]);

        let conditions = PathCondition::analyze(&ir_method(entry, blocks));

        assert!(!conditions.contains_key(&impossible));
        assert_eq!(conditions[&reachable], PathCondition::one());
    }

    #[test]
    fn constant_definition_unifies_guards_across_blocks() {
        let [entry, first_taken, first_otherwise, impossible, reachable] = ids(0);
        let [zero, unknown]: [ValueId; 2] = ids(0);
        let first = Predicate::Equal(unknown.into(), zero.into());
        let second = Predicate::IsZero(unknown.into());
        let blocks = HashMap::from([
            bb(
                entry,
                [],
                &[def(zero, Expression::Const(Integer(0)))],
                branch(
                    arm(
                        first_taken,
                        [],
                        Transfer::Conditional(BranchGuard::of(first.into())),
                    ),
                    edge(first_otherwise, []),
                ),
            ),
            code(
                first_taken,
                branch(
                    arm(
                        impossible,
                        [],
                        Transfer::Conditional(BranchGuard::of(!BooleanVariable::from(second))),
                    ),
                    edge(reachable, []),
                ),
            ),
            code(first_otherwise, void()),
            code(impossible, void()),
            code(reachable, void()),
        ]);

        let conditions = PathCondition::analyze(&ir_method(entry, blocks));

        assert!(!conditions.contains_key(&impossible));
        assert!(conditions.contains_key(&reachable));
    }

    #[test]
    fn constant_folding_preserves_canonical_literal_polarity() {
        let integer = |value| PathValue::Constant(Integer(value));
        let cases = [
            (
                BooleanVariable::Positive(Predicate::GreaterThanOrEqual(integer(1), integer(2))),
                false,
            ),
            (
                BooleanVariable::Positive(Predicate::NotEqual(integer(0), integer(1))),
                true,
            ),
            (
                BooleanVariable::Positive(Predicate::IsNotNull(PathValue::Constant(Null))),
                false,
            ),
            (
                BooleanVariable::Negative(Predicate::IsNonZero(integer(0))),
                true,
            ),
        ];

        for (literal, expected_taken) in cases {
            let [entry, taken, otherwise] = ids(0);
            let blocks = HashMap::from([
                code(
                    entry,
                    branch(
                        arm(taken, [], Transfer::Conditional(BranchGuard::of(literal))),
                        edge(otherwise, []),
                    ),
                ),
                code(taken, void()),
                code(otherwise, void()),
            ]);
            let conditions = PathCondition::analyze(&ir_method(entry, blocks));

            assert_eq!(conditions.contains_key(&taken), expected_taken);
        }
    }
}
