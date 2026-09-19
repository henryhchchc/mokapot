use super::*;
use crate::ir::{
    MokaIRFrameError,
    expression::{Conversion, Expression, MathOperation},
};

#[test]
fn valid_stack_shuffles_preserve_value_identity_and_order() {
    let dup_method = method(
        [
            (0, Instruction::ILoad0),
            (1, Instruction::Dup),
            (2, Instruction::IAdd),
            (3, Instruction::IReturn),
        ],
        "(I)I",
        vec![],
    );
    let ir = build(&dup_method).unwrap();
    let operation = ir
        .blocks()
        .flat_map(|(_, block)| &block.operations)
        .next()
        .unwrap();

    assert!(matches!(
        operation,
        Operation::Definition {
            expr: Expression::Math(MathOperation::Add(lhs, rhs)),
            ..
        } if lhs == rhs && *lhs == ir.parameter_values()[0]
    ));

    let category_2_method = method(
        [
            (0, Instruction::LLoad0),
            (1, Instruction::Dup2),
            (2, Instruction::LAdd),
            (3, Instruction::LReturn),
        ],
        "(J)J",
        vec![],
    );
    let ir = build(&category_2_method).unwrap();
    let operation = ir
        .blocks()
        .flat_map(|(_, block)| &block.operations)
        .next()
        .unwrap();
    assert!(matches!(
        operation,
        Operation::Definition {
            expr: Expression::Math(MathOperation::Add(lhs, rhs)),
            ..
        } if lhs == rhs && *lhs == ir.parameter_values()[0]
    ));

    let mixed_method = method(
        [
            (0, Instruction::LLoad0),
            (1, Instruction::ILoad2),
            (2, Instruction::DupX2),
            (3, Instruction::Pop),
            (4, Instruction::L2I),
            (5, Instruction::IAdd),
            (6, Instruction::IReturn),
        ],
        "(JI)I",
        vec![],
    );
    let ir = build(&mixed_method).unwrap();
    let mut operations = ir.blocks().flat_map(|(_, block)| &block.operations);
    let conversion = operations.next().unwrap();
    let conversion_value = conversion.def().unwrap();
    assert!(matches!(
        conversion,
        Operation::Definition {
            expr: Expression::Conversion(Conversion::Long2Int(value)),
            ..
        } if *value == ir.parameter_values()[0]
    ));
    let addition = operations.next().unwrap();
    assert!(matches!(
        addition,
        Operation::Definition {
            expr: Expression::Math(MathOperation::Add(lhs, rhs)),
            ..
        } if *lhs == ir.parameter_values()[1] && *rhs == conversion_value
    ));
    assert!(operations.next().is_none());
}

#[test]
fn invalid_stack_shuffles_report_the_source_instruction() {
    enum Failure {
        Underflow,
        InvalidLayout,
    }

    let cases = [
        (
            vec![(0, Instruction::Dup), (1, Instruction::Return)],
            "()V",
            0.into(),
            Failure::Underflow,
        ),
        (
            vec![
                (0, Instruction::LLoad0),
                (1, Instruction::Dup),
                (2, Instruction::Return),
            ],
            "(J)V",
            1.into(),
            Failure::InvalidLayout,
        ),
    ];

    for (instructions, descriptor, expected_pc, failure) in cases {
        let result = build(&method(instructions, descriptor, vec![]));
        assert!(match (failure, result) {
            (
                Failure::Underflow,
                Err(MokaIRBuildError::InvalidFrame {
                    pc: Some(pc),
                    source: MokaIRFrameError::StackUnderflow,
                }),
            )
            | (
                Failure::InvalidLayout,
                Err(MokaIRBuildError::InvalidFrame {
                    pc: Some(pc),
                    source: MokaIRFrameError::InvalidSlotLayout,
                }),
            ) => pc == expected_pc,
            _ => false,
        });
    }
}

#[test]
fn array_write_is_an_effect_without_a_definition() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::ILoad1),
            (2, Instruction::ILoad2),
            (3, Instruction::IAStore),
            (4, Instruction::Return),
        ],
        "([III)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let effect = ir
        .source_map()
        .instructions_at(3.into())
        .find_map(|location| match ir.instruction(location) {
            Some(InstructionRef::Terminator(terminator)) => terminator.operation(),
            Some(InstructionRef::BlockParameter(_) | InstructionRef::Operation(_)) | None => None,
        })
        .unwrap();

    assert!(matches!(effect, Operation::Effect { .. }));
    assert_eq!(effect.def(), None);
    assert_eq!(effect.uses().len(), 3);
    assert_eq!(ir.source_map().instructions_at(0.into()).count(), 0);
    assert_eq!(ir.source_map().instructions_at(1.into()).count(), 0);
    assert_eq!(ir.source_map().instructions_at(2.into()).count(), 0);
}

#[test]
fn monitor_operations_are_effects_without_definitions() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::MonitorEnter),
            (2, Instruction::ALoad0),
            (3, Instruction::MonitorExit),
            (4, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let instructions = ir
        .blocks()
        .filter_map(|(_, block)| block.terminator.operation())
        .collect::<Vec<_>>();

    assert_eq!(instructions.len(), 2);
    assert!(instructions.iter().all(|instruction| {
        instruction.def().is_none() && matches!(instruction, Operation::Effect { .. })
    }));
}
