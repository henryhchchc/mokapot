use super::*;
use crate::ir::expression::{Conversion, MathOperation};

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
        (0, InstructionData::ILoad0),
        (1, InstructionData::Dup),
        (2, InstructionData::IAdd),
        (3, InstructionData::IReturn),
    ];
    let ir = build(&method(body, "(I)I", vec![])).unwrap();
    let parameter = ir.parameters[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    let expected = Expression::Math(MathOperation::Add(parameter, parameter));
    assert_eq!(expr, &expected);

    let body = [
        (0, InstructionData::LLoad0),
        (1, InstructionData::Dup2),
        (2, InstructionData::LAdd),
        (3, InstructionData::LReturn),
    ];
    let ir = build(&method(body, "(J)J", vec![])).unwrap();
    let parameter = ir.parameters[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    let expected = Expression::Math(MathOperation::Add(parameter, parameter));
    assert_eq!(expr, &expected);

    let body = [
        (0, InstructionData::LLoad0),
        (1, InstructionData::ILoad2),
        (2, InstructionData::DupX2),
        (3, InstructionData::Pop),
        (4, InstructionData::L2I),
        (5, InstructionData::IAdd),
        (6, InstructionData::IReturn),
    ];
    let ir = build(&method(body, "(JI)I", vec![])).unwrap();
    let mut operations = operations(&ir);
    let (conversion, expr) = definition(operations.next().unwrap());
    let expected = Expression::Conversion(Conversion::Long2Int(ir.parameters[0]));
    assert_eq!(expr, &expected);
    let (_, expr) = definition(operations.next().unwrap());
    let expected = Expression::Math(MathOperation::Add(ir.parameters[1], conversion));
    assert_eq!(expr, &expected);
    assert!(operations.next().is_none());
}

#[test]
fn invalid_stack_shuffles_report_the_source_instruction() {
    let body = [(0, InstructionData::Dup), (1, InstructionData::Return)];
    let underflow = build_failure(&method(body, "()V", vec![]));
    assert_matches!(underflow, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 0.into());

    let body = [
        (0, InstructionData::LLoad0),
        (1, InstructionData::Dup),
        (2, InstructionData::Return),
    ];
    let layout = build_failure(&method(body, "(J)V", vec![]));
    assert_matches!(layout, (Some(pc), MokaIRBuildErrorKind::InvalidSlotLayout) if pc == 1.into());
}

#[test]
fn instance_invocations_require_a_receiver() {
    let method_ref = method_ref("accept", "(IJ)V");
    let mut pool = ConstantPool::new();
    let target = pool.insert_method_ref(method_ref);
    for invocation in [
        InstructionData::InvokeVirtual(target),
        InstructionData::InvokeInterface(target, 4),
        InstructionData::InvokeSpecial(target),
    ] {
        let body = [
            (0, InstructionData::ILoad0),
            (1, InstructionData::LLoad1),
            (2, invocation),
            (3, InstructionData::Return),
        ];
        let method = method_with_pool(pool.clone(), body, "(IJ)V", vec![]);
        let failure = build_failure(&method);
        assert_matches!(failure, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 2.into());
    }
}
