use super::*;
use crate::{
    ir::expression::{ArrayOperation, Conversion, FieldAccess, LockOperation, MathOperation},
    jvm::references::FieldRef,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};

/// Returns the value and expression of a definition operation.
fn definition(operation: &Operation) -> (ValueId, &Expression) {
    let Operation::Definition { value, expr } = operation else {
        panic!("the operation must define a value");
    };
    (*value, expr)
}

fn assert_returns(ir: &MokaIRMethod, pc: u16, value: ValueId) {
    assert_matches!(terminator_at(ir, pc.into()), Terminator::Return { value: Some(returned), .. } if *returned == value);
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
    let parameter = ir.parameters[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    let expected = Expression::Math(MathOperation::Add(parameter, parameter));
    assert_eq!(expr, &expected);

    let body = [
        (0, Instruction::LLoad0),
        (1, Instruction::Dup2),
        (2, Instruction::LAdd),
        (3, Instruction::LReturn),
    ];
    let ir = build(&method(body, "(J)J", vec![])).unwrap();
    let parameter = ir.parameters[0];
    let (_, expr) = definition(operations(&ir).next().unwrap());
    let expected = Expression::Math(MathOperation::Add(parameter, parameter));
    assert_eq!(expr, &expected);

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
    let body = [(0, Instruction::Dup), (1, Instruction::Return)];
    let underflow = build_failure(&method(body, "()V", vec![]));
    assert_matches!(underflow, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 0.into());

    let body = [
        (0, Instruction::LLoad0),
        (1, Instruction::Dup),
        (2, Instruction::Return),
    ];
    let layout = build_failure(&method(body, "(J)V", vec![]));
    assert_matches!(layout, (Some(pc), MokaIRBuildErrorKind::InvalidSlotLayout) if pc == 1.into());
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
    let operation = terminator_at(&ir, 3.into()).operation().unwrap();

    let expected = effect(Expression::Array(ArrayOperation::Write {
        array_ref: ir.parameters[0],
        index: ir.parameters[1],
        value: ir.parameters[2],
    }));
    assert_eq!(operation, &expected);
    for pc in [0, 1, 2] {
        assert_eq!(ir.source_map.instructions_at(pc.into()).count(), 0);
    }
}

#[test]
fn multidimensional_array_lengths_follow_array_nesting_order() {
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::ILoad1),
        (2, Instruction::MultiANewArray(ref_t("[[I"), 2)),
        (3, Instruction::AReturn),
    ];
    let ir = build(&method(body, "(II)[[I", vec![])).unwrap();
    let operation = terminator_at(&ir, 2.into()).operation().unwrap();
    let (_, Expression::Array(ArrayOperation::NewMultiDim { dimensions, .. })) =
        definition(operation)
    else {
        panic!("multianewarray must define a multidimensional array");
    };

    assert_eq!(dimensions, &ir.parameters);
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
    let instructions = terminator_operations(&ir).cloned().collect::<Vec<_>>();

    assert_eq!(instructions.len(), 2);
    let expected = [
        effect(LockOperation::Acquire(ir.parameters[0])),
        effect(LockOperation::Release(ir.parameters[0])),
    ];
    assert_eq!(instructions, expected);
}

#[test]
fn invocation_kinds_preserve_receivers_arguments_and_return_categories() {
    let kinds: [fn(ValueId) -> InvocationKind; 4] = [
        |_| InvocationKind::Static,
        |this| InvocationKind::Virtual { this },
        |this| InvocationKind::Interface { this },
        |this| InvocationKind::Special { this },
    ];
    for (descriptor, caller_descriptor, return_instruction) in [
        ("(IJ)V", "(Ljava/lang/Object;IJ)V", Instruction::Return),
        ("(IJ)I", "(Ljava/lang/Object;IJ)I", Instruction::IReturn),
        ("(IJ)J", "(Ljava/lang/Object;IJ)J", Instruction::LReturn),
    ] {
        let method_ref = method_ref("accept", descriptor);
        let invocations = [
            Instruction::InvokeStatic(method_ref.clone()),
            Instruction::InvokeVirtual(method_ref.clone()),
            Instruction::InvokeInterface(method_ref.clone(), 4),
            Instruction::InvokeSpecial(method_ref.clone()),
        ];
        for (invocation, kind) in invocations.into_iter().zip(kinds) {
            let receiver = if matches!(invocation, Instruction::InvokeStatic(_)) {
                Instruction::Nop
            } else {
                Instruction::ALoad0
            };
            let body = [
                (0, receiver),
                (1, Instruction::ILoad1),
                (2, Instruction::LLoad2),
                (3, invocation),
                (4, return_instruction.clone()),
            ];
            let ir = lift(body, caller_descriptor, vec![]);
            let operation = terminator_at(&ir, 3.into()).operation().unwrap();
            let kind = kind(ir.parameters[0]);
            let args = ir.parameters[1..].iter().copied();
            let expected = call(kind, method_ref.clone(), args);
            if matches!(return_instruction, Instruction::Return) {
                assert_eq!(operation, &effect(expected));
            } else {
                let (value, expression) = definition(operation);
                assert_eq!(expression, &expected);
                assert_returns(&ir, 4, value);
            }
        }
    }
}

#[test]
fn instance_invocations_require_a_receiver() {
    let method_ref = method_ref("accept", "(IJ)V");
    for invocation in [
        Instruction::InvokeVirtual(method_ref.clone()),
        Instruction::InvokeInterface(method_ref.clone(), 4),
        Instruction::InvokeSpecial(method_ref),
    ] {
        let body = [
            (0, Instruction::ILoad0),
            (1, Instruction::LLoad1),
            (2, invocation),
            (3, Instruction::Return),
        ];
        let method = method(body, "(IJ)V", vec![]);
        let failure = build_failure(&method);
        assert_matches!(failure, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 2.into());
    }
}

#[test]
fn dynamic_invocations_remain_closures() {
    let descriptor: MethodDescriptor = "(IJ)Ljava/lang/Object;".parse().unwrap();
    let invocation = Instruction::InvokeDynamic {
        bootstrap_method_index: 7,
        name: "accept".to_owned(),
        descriptor: descriptor.clone(),
    };
    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::LLoad1),
        (2, invocation),
        (3, Instruction::AReturn),
    ];
    let ir = lift(body, "(IJ)Ljava/lang/Object;", vec![]);
    let operation = terminator_at(&ir, 2.into()).operation().unwrap();
    let (value, expression) = definition(operation);
    let expected = Expression::Closure {
        name: "accept".to_owned(),
        captures: ir.parameters.clone(),
        bootstrap_method_index: 7,
        closure_descriptor: descriptor,
    };
    assert_eq!(expression, &expected);
    assert_returns(&ir, 3, value);
}

#[test]
fn field_reads_define_values_and_writes_are_effects() {
    let field = FieldRef {
        owner: ref_t("java/lang/Object"),
        name: "count".to_owned(),
        field_type: FieldType::Base(PrimitiveType::Int),
    };
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::GetField(field.clone())),
        (2, Instruction::IReturn),
    ];
    let read = lift(body, "(Ljava/lang/Object;)I", vec![]);
    let operation = terminator_at(&read, 1.into()).operation().unwrap();
    let (value, expression) = definition(operation);
    let expected = Expression::Field(FieldAccess::ReadInstance {
        object_ref: read.parameters[0],
        field: field.clone(),
    });
    assert_eq!(expression, &expected);
    assert_returns(&read, 2, value);

    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::ILoad1),
        (2, Instruction::PutField(field.clone())),
        (3, Instruction::Return),
    ];
    let write = lift(body, "(Ljava/lang/Object;I)V", vec![]);
    let operation = terminator_at(&write, 2.into()).operation().unwrap();
    let expected = effect(FieldAccess::WriteInstance {
        object_ref: write.parameters[0],
        field: field.clone(),
        value: write.parameters[1],
    });
    assert_eq!(operation, &expected);

    let body = [
        (0, Instruction::GetStatic(field.clone())),
        (1, Instruction::IReturn),
    ];
    let read = lift(body, "()I", vec![]);
    let (value, expression) = definition(terminator_at(&read, 0.into()).operation().unwrap());
    let expected = Expression::Field(FieldAccess::ReadStatic {
        field: field.clone(),
    });
    assert_eq!(expression, &expected);
    assert_returns(&read, 1, value);

    let body = [
        (0, Instruction::ILoad0),
        (1, Instruction::PutStatic(field.clone())),
        (2, Instruction::Return),
    ];
    let write = lift(body, "(I)V", vec![]);
    let expected = effect(FieldAccess::WriteStatic {
        field,
        value: write.parameters[0],
    });
    assert_eq!(terminator_at(&write, 1.into()).operation(), Some(&expected));
}

#[test]
fn array_read_uses_index_then_returns_the_defined_value() {
    let body = [
        (0, Instruction::ALoad0),
        (1, Instruction::ILoad1),
        (2, Instruction::IALoad),
        (3, Instruction::IReturn),
    ];
    let ir = lift(body, "([II)I", vec![]);
    let operation = terminator_at(&ir, 2.into()).operation().unwrap();
    let (value, expression) = definition(operation);
    let expected = Expression::Array(ArrayOperation::Read {
        array_ref: ir.parameters[0],
        index: ir.parameters[1],
    });
    assert_eq!(expression, &expected);
    assert_returns(&ir, 3, value);
}
