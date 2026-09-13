use super::{
    analyzer::JvmSymbolicExecutor,
    fact::{FrameMergeSite, SymbolicValue},
    solver::merge_frame_at,
};
use crate::{
    ir::generator::{
        identity::SsaValueId,
        jvm::{
            frame::{Entry, FrameSlot, JvmStackFrame},
            normalization::Location,
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
) -> JvmStackFrame<SymbolicValue> {
    let parameters = parameters
        .iter()
        .copied()
        .map(SymbolicValue::Value)
        .collect::<Vec<_>>();
    JvmStackFrame::with_inputs(
        descriptor,
        max_locals,
        max_stack,
        this_value.map(SymbolicValue::Value),
        &parameters,
    )
    .expect("frame fits descriptor")
}

#[test]
fn merge_identity_is_stable_for_a_location_and_slot() {
    let descriptor = "(I)V".parse().expect("valid descriptor");
    let location = Location::entry(0.into());
    let frame = |value| frame_with_inputs(&descriptor, 1, 0, None, &[SsaValueId::new(value)]);
    let mut merged = frame(1);

    assert!(merge_frame_at(location, &mut merged, frame(2)));
    let expected = SymbolicValue::Merged(FrameMergeSite {
        location,
        slot: FrameSlot::Local(0),
    });
    assert_eq!(merged.local_variables(), &[Entry::Value(expected)]);
    assert!(!merge_frame_at(location, &mut merged, frame(3)));
    assert_eq!(merged.local_variables(), &[Entry::Value(expected)]);
}

#[test]
fn frame_merge_is_permutation_independent() {
    let descriptor = "(I)V".parse().expect("valid descriptor");
    let location = Location::entry(0.into());
    let frame = |value| frame_with_inputs(&descriptor, 1, 0, None, &[SsaValueId::new(value)]);
    let expected = [Entry::Value(SymbolicValue::Merged(FrameMergeSite {
        location,
        slot: FrameSlot::Local(0),
    }))];

    for order in [
        [1, 2, 3],
        [1, 3, 2],
        [2, 1, 3],
        [2, 3, 1],
        [3, 1, 2],
        [3, 2, 1],
    ] {
        let mut merged = frame(order[0]);
        merge_frame_at(location, &mut merged, frame(order[1]));
        merge_frame_at(location, &mut merged, frame(order[2]));
        assert_eq!(merged.local_variables(), expected);
    }
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
    let mut analyzer = JvmSymbolicExecutor::for_method(&method).expect("valid method");
    analyzer.solve_locations().expect("valid loop");

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

#[test]
fn reprocessing_replaces_stale_predecessor_output() {
    let method = method(
        [
            (0.into(), JvmInstruction::IConst0),
            (1.into(), JvmInstruction::Nop),
            (2.into(), JvmInstruction::Pop),
            (3.into(), JvmInstruction::IConst1),
            (4.into(), JvmInstruction::Goto(1.into())),
        ],
        "()V",
        vec![],
    );
    let mut analyzer = JvmSymbolicExecutor::for_method(&method).expect("valid method");
    let nodes = analyzer.solve_locations().expect("valid loop");

    assert_eq!(
        nodes[&Location::entry(2.into())]
            .incoming_frame
            .operand_stack(),
        &[Entry::Value(SymbolicValue::Merged(FrameMergeSite {
            location: Location::entry(1.into()),
            slot: FrameSlot::Stack(0),
        }))]
    );
}
