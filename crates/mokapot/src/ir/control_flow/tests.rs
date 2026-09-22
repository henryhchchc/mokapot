use std::collections::HashMap;

use super::*;
use crate::{
    ir::{
        NumericalId,
        control_flow::{
            ControlTransfer as Transfer,
            path_condition::{BooleanVariable, SolvingBudget, analyze},
        },
        expression::Predicate,
        test::prelude::*,
    },
    jvm::ConstantValue::Null,
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
    let conditions = analyze(&blocks, b0, SolvingBudget::default());

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
    let conditions = analyze(&blocks, entry_id, SolvingBudget::default());

    assert_eq!(conditions[&tried], conditions[&normal]);
    assert_eq!(conditions[&tried], conditions[&handler]);
    assert_ne!(conditions[&tried], conditions[&otherwise]);
}
