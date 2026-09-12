use std::collections::BTreeMap;

use super::{
    analyzer::JvmFrameAnalyzer,
    fact::{JvmFrameFact, MergeIdentity, OperandState},
};
use crate::{
    analysis::fixed_point::{JoinSemiLattice, solve},
    ir::generator::{
        identity::SsaValueId,
        jvm::{
            frame::{Entry, FrameSlot, JvmStackFrame},
            normalization::{Location, ReturnAddress},
        },
        tests::method,
    },
    jvm::code::Instruction as JvmInstruction,
    types::method_descriptor::MethodDescriptor,
};

fn frame_with_inputs(
    descriptor: &MethodDescriptor,
    max_locals: u16,
    max_stack: u16,
    this_value: Option<SsaValueId>,
    parameters: &[SsaValueId],
) -> JvmStackFrame<OperandState> {
    let parameters = parameters
        .iter()
        .copied()
        .map(OperandState::Value)
        .collect::<Vec<_>>();
    JvmStackFrame::with_inputs(
        descriptor,
        max_locals,
        max_stack,
        this_value.map(OperandState::Value),
        &parameters,
    )
    .expect("frame fits descriptor")
}

#[test]
fn merge_identity_is_stable_for_a_location_and_slot() {
    let descriptor = "(I)V".parse().expect("valid descriptor");
    let location = Location::entry(0.into());
    let frame = |value| frame_with_inputs(&descriptor, 1, 0, None, &[SsaValueId::new(value)]);
    let mut merged = JvmFrameFact::new(location, frame(1));

    assert!(merged.join_assign(JvmFrameFact::new(location, frame(2))));
    let expected = OperandState::Merged(MergeIdentity {
        location,
        slot: FrameSlot::Local(0),
    });
    assert_eq!(merged.frame.local_variables(), &[Entry::Value(expected)]);
    assert!(!merged.join_assign(JvmFrameFact::new(location, frame(3))));
    assert_eq!(merged.frame.local_variables(), &[Entry::Value(expected)]);
}

#[test]
fn unwind_facts_discard_irrelevant_values_before_merging() {
    let descriptor = "(I)V".parse().expect("valid descriptor");
    let frame = |value| frame_with_inputs(&descriptor, 1, 0, None, &[SsaValueId::new(value)]);
    let mut unwind = JvmFrameFact::new(Location::Unwind, frame(1));

    assert!(unwind.frame.values().next().is_none());
    assert!(!unwind.join_assign(JvmFrameFact::new(Location::Unwind, frame(2))));
    assert!(unwind.frame.values().next().is_none());
}

#[test]
fn context_free_value_conflict_is_invalid() {
    let lhs = Entry::Value(OperandState::Value(SsaValueId::new(0)));
    let rhs = Entry::Value(OperandState::Value(SsaValueId::new(1)));

    assert_eq!(lhs.join(rhs), Entry::Value(OperandState::Invalid));
}

#[test]
fn context_free_same_value_is_unchanged() {
    let value = OperandState::Value(SsaValueId::new(0));

    assert_eq!(
        Entry::Value(value).join(Entry::Value(value)),
        Entry::Value(value)
    );
}

#[test]
fn incompatible_legacy_values_become_invalid() {
    let value = Entry::Value(OperandState::Value(SsaValueId::new(0)));
    let address = Entry::Value(OperandState::ReturnAddress(ReturnAddress::for_test(0)));

    assert_eq!(value.join(address), Entry::Value(OperandState::Invalid));
}

#[test]
fn a_value_missing_on_one_path_is_unavailable() {
    let value = Entry::Value(OperandState::Value(SsaValueId::new(0)));

    assert_eq!(
        value.clone().join(Entry::UninitializedLocal),
        Entry::UninitializedLocal
    );
    assert_eq!(
        Entry::UninitializedLocal.join(value),
        Entry::UninitializedLocal
    );
}

#[test]
fn reprocessing_loop_allocates_identities_only_for_definitions() {
    let method = method(
        [
            (0.into(), JvmInstruction::IConst0),
            (1.into(), JvmInstruction::IStore0),
            (2.into(), JvmInstruction::ILoad0),
            (3.into(), JvmInstruction::IConst1),
            (4.into(), JvmInstruction::IAdd),
            (5.into(), JvmInstruction::IStore0),
            (6.into(), JvmInstruction::Goto(2.into())),
        ],
        "()V",
        vec![],
    );
    let mut analyzer = JvmFrameAnalyzer::for_method(&method).expect("valid method");
    let _: BTreeMap<Location, JvmFrameFact> = solve(&mut analyzer).expect("valid loop");

    assert_eq!(analyzer.definition_ids.len(), 3);
    assert_eq!(analyzer.value_id_allocator.next_value_idx, 3);
    assert!(
        analyzer
            .definition_ids
            .contains_key(&Location::entry(0.into()))
    );
    assert!(
        analyzer
            .definition_ids
            .contains_key(&Location::entry(3.into()))
    );
    assert!(
        analyzer
            .definition_ids
            .contains_key(&Location::entry(4.into()))
    );
    assert!(
        !analyzer
            .definition_ids
            .contains_key(&Location::entry(1.into()))
    );
    assert!(
        !analyzer
            .definition_ids
            .contains_key(&Location::entry(2.into()))
    );
    assert!(
        !analyzer
            .definition_ids
            .contains_key(&Location::entry(5.into()))
    );
    assert!(
        !analyzer
            .definition_ids
            .contains_key(&Location::entry(6.into()))
    );
}
