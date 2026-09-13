use super::materialize::classify_block_end;
use crate::{
    ir::{
        OperationKind, TerminatorKind,
        expression::Expression,
        generator::{
            identity::SsaValueId,
            jvm::{
                instruction::RegisterInstruction,
                subroutine_expansion::{Location, ReturnAddress},
            },
        },
    },
    jvm::ConstantValue,
};

#[test]
fn pseudo_and_legacy_instructions_become_semantic_gotos() {
    let instructions = [
        RegisterInstruction::HandlerEntry,
        RegisterInstruction::Erased,
        RegisterInstruction::Subroutine {
            target: Location::Unwind,
        },
        RegisterInstruction::SubroutineReturn(
            crate::ir::generator::jvm::symbolic_execution::Value::ReturnAddress(
                ReturnAddress::for_test(0),
            ),
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
            value: crate::ir::generator::jvm::symbolic_execution::Value::Ssa(actual),
            ..
        }) if actual == value
    ));
    assert_eq!(terminator, TerminatorKind::Fallible);
}
