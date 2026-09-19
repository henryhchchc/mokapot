use super::*;
use crate::ir::{
    MokaIRFrameError, ValueId,
    expression::{Conversion, Expression, MathOperation},
};

/// Returns the value and expression of a definition operation.
fn definition(operation: &Operation) -> (ValueId, &Expression) {
    let Operation::Definition { value, expr } = operation else {
        panic!("the operation must define a value");
    };
    (*value, expr)
}

#[test]
fn valid_stack_shuffles_preserve_value_identity_and_order() {
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::Dup),
        (2, Instruction::IAdd),
        (3, Instruction::IReturn),
    ];
    let ir = build(&method(body, "(I)I", vec![])).unwrap();
    let parameter = ir.parameter_values()[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    assert_eq!(
        expr,
        &Expression::Math(MathOperation::Add(parameter, parameter))
    );

    let body = [
        (0, Instruction::LLoad0),
        (1, Instruction::Dup2),
        (2, Instruction::LAdd),
        (3, Instruction::LReturn),
    ];
    let ir = build(&method(body, "(J)J", vec![])).unwrap();
    let parameter = ir.parameter_values()[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    assert_eq!(
        expr,
        &Expression::Math(MathOperation::Add(parameter, parameter))
    );

    let body = [
        (0, Instruction::LLoad0),
        (1, Instruction::ILoad2),
        (2, Instruction::DupX2),
        (3, Instruction::Pop),
        (4, Instruction::L2I),
        (5, Instruction::IAdd),
        (6, Instruction::IReturn),
    ];
    let ir = build(&method(body, "(JI)I", vec![])).unwrap();
    let parameters = ir.parameter_values();
    let mut operations = operations(&ir);
    let (conversion, expr) = definition(operations.next().unwrap());
    assert_eq!(
        expr,
        &Expression::Conversion(Conversion::Long2Int(parameters[0]))
    );
    let (_, expr) = definition(operations.next().unwrap());
    assert_eq!(
        expr,
        &Expression::Math(MathOperation::Add(parameters[1], conversion))
    );
    assert!(operations.next().is_none());
}

#[test]
fn invalid_stack_shuffles_report_the_source_instruction() {
    let body = vec![(0, Instruction::Dup), (1, Instruction::Return)];
    let underflow = frame_failure(&method(body, "()V", vec![]));
    assert!(matches!(underflow, (Some(pc), MokaIRFrameError::StackUnderflow) if pc == 0.into()));

    let body = vec![
        (0, Instruction::LLoad0),
        (1, Instruction::Dup),
        (2, Instruction::Return),
    ];
    let layout = frame_failure(&method(body, "(J)V", vec![]));
    assert!(matches!(layout, (Some(pc), MokaIRFrameError::InvalidSlotLayout) if pc == 1.into()));
}

#[test]
fn array_write_is_an_effect_without_a_definition() {
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::ILoad1),
        (2, Instruction::ILoad2),
        (3, Instruction::IAStore),
        (4, Instruction::Return),
    ];
    let ir = build(&method(body, "([III)V", vec![])).unwrap();
    let effect = ir
        .source_map()
        .instructions_at(3.into())
        .find_map(|it| match ir.instruction(it) {
            Some(InstructionRef::Terminator(terminator)) => terminator.operation(),
            _ => None,
        })
        .unwrap();

    assert!(matches!(effect, Operation::Effect { .. }));
    assert_eq!(effect.def(), None);
    assert_eq!(effect.uses().len(), 3);
    for pc in [0, 1, 2] {
        assert_eq!(ir.source_map().instructions_at(pc.into()).count(), 0);
    }
}

#[test]
fn monitor_operations_are_effects_without_definitions() {
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::MonitorEnter),
        (2, Instruction::ALoad0),
        (3, Instruction::MonitorExit),
        (4, Instruction::Return),
    ];
    let ir = build(&method(body, "(Ljava/lang/Object;)V", vec![])).unwrap();
    let instructions = terminator_operations(&ir).collect::<Vec<_>>();

    assert_eq!(instructions.len(), 2);
    assert!(
        instructions
            .iter()
            .all(|it| { it.def().is_none() && matches!(it, Operation::Effect { .. }) })
    );
}
