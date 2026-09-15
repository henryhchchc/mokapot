use super::{Arm, BlockEnd, materialize::classify_block_end};
use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        expression::Expression,
        generator::{
            bytecode_analysis::{
                self, Edge, Node, NodeAddress, RegisterInstruction, ReturnAddress, jvm::Frame,
            },
            error::Error,
            identity::SsaValueId,
        },
    },
    jvm::ConstantValue,
};

/// Builds a frame that fits a method without parameters.
fn entry_frame() -> Frame<bytecode_analysis::Value> {
    Frame::for_method_entry(&"()V".parse().expect("valid descriptor"), 0, 0, None, &[])
        .expect("frame fits descriptor")
}

#[test]
fn pseudo_and_legacy_instructions_become_semantic_gotos() {
    let instructions = [
        RegisterInstruction::HandlerEntry,
        RegisterInstruction::Erased,
        RegisterInstruction::Subroutine {
            target: NodeAddress::Unwind,
        },
        RegisterInstruction::SubroutineReturn(
            crate::ir::generator::bytecode_analysis::Value::ReturnAddress(ReturnAddress::for_test(
                0,
            )),
        ),
    ];

    for instruction in instructions {
        let (operation, terminator) = classify_block_end(instruction, false);
        assert!(operation.is_none());
        assert_eq!(terminator, TerminatorKind::Goto);
    }
}

#[test]
fn fallible_definition_becomes_an_operation_and_terminator() {
    let value = SsaValueId::new(7);
    let (operation, terminator) = classify_block_end(
        RegisterInstruction::Definition {
            value,
            expr: Expression::Const(ConstantValue::Integer(1)),
        },
        true,
    );

    assert!(matches!(
        operation,
        Some(OperationKind::Definition {
            value: crate::ir::generator::bytecode_analysis::Value::Ssa(actual),
            ..
        }) if actual == value
    ));
    assert_eq!(terminator, TerminatorKind::Fallible);
}

#[test]
fn only_an_ordinary_fallthrough_elides_into_its_successor() {
    let frame = entry_frame();
    let next = NodeAddress::entry(1.into());
    let elsewhere = NodeAddress::entry(2.into());
    let edge = |target| Edge {
        target,
        transfer: ControlTransfer::Unconditional,
        target_frame: frame.clone(),
    };
    let node = |instruction, outgoing_edges| Node {
        incoming_frame: frame.clone(),
        instruction,
        outgoing_edges,
    };

    assert!(node(RegisterInstruction::Erased, vec![edge(next)]).elides_into(next));

    // A transfer ends its block even when its edge is the fallthrough.
    assert!(
        !node(
            RegisterInstruction::Jump {
                condition: None,
                target: 1.into()
            },
            vec![edge(next)]
        )
        .elides_into(next)
    );

    // So does an exceptional exit, and an edge that is not the fallthrough.
    assert!(
        !node(
            RegisterInstruction::Erased,
            vec![Edge {
                target: next,
                transfer: ControlTransfer::Unwind,
                target_frame: frame.clone(),
            }]
        )
        .elides_into(next)
    );
    assert!(!node(RegisterInstruction::Erased, vec![edge(next)]).elides_into(elsewhere));
    assert!(
        !node(
            RegisterInstruction::Erased,
            vec![edge(next), edge(elsewhere)]
        )
        .elides_into(next)
    );
}

#[test]
fn block_end_rejects_mismatched_terminator_and_arm_shapes() {
    let arm = |transfer| Arm {
        target: BlockId::new(0),
        transfer,
        frame: entry_frame(),
    };

    assert!(
        BlockEnd::new(
            TerminatorKind::Goto,
            None,
            vec![arm(ControlTransfer::Exception(None))],
        )
        .is_err_and(|error| matches!(
            error,
            Error::InternalInvariant {
                message: "a terminator has incompatible control-flow arms",
                ..
            }
        ))
    );
    assert!(
        BlockEnd::new(
            TerminatorKind::Fallible,
            None,
            vec![arm(ControlTransfer::Unconditional)],
        )
        .is_err()
    );
    assert!(
        BlockEnd::new(
            TerminatorKind::Return(None),
            None,
            vec![arm(ControlTransfer::Unconditional)],
        )
        .is_err()
    );
    assert!(
        BlockEnd::new(
            TerminatorKind::Throw(bytecode_analysis::Value::Ssa(SsaValueId::new(0))),
            None,
            Vec::new(),
        )
        .is_err()
    );
}
