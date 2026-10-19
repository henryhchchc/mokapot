use super::*;
use mokapot::ir::{
    Operation, ValueId,
    expression::{
        ArrayOperation, Conversion, Expression, FieldAccess, InvocationKind, LockOperation,
        MathOperation,
    },
};

fn method_ir(class_name: &str, name: &str) -> MokaIRMethod {
    let bytes = support::class_bytes(&format!("org/mokapot/test/{class_name}"));
    let class = Class::from_bytes(&bytes).unwrap();
    let method = class
        .methods()
        .find(|method| method.name() == name)
        .unwrap();
    MokaIRMethod::from_method(method).unwrap()
}

fn operations(ir: &MokaIRMethod) -> Vec<&Operation> {
    let mut operations = Vec::new();
    for (_, block) in reachable_blocks(ir) {
        operations.extend(&block.operations);
        operations.extend(block.terminator.operation());
    }
    operations
}

fn definition<'ir>(operations: &[&'ir Operation], value: ValueId) -> &'ir Expression {
    operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Definition {
                value: defined,
                expr,
            } if *defined == value => Some(expr),
            Operation::Definition { .. } | Operation::Effect { .. } => None,
        })
        .expect("the value has a definition")
}

fn adds_one(operations: &[&Operation], expression: &Expression, value: ValueId) -> bool {
    match expression {
        Expression::Math(MathOperation::Increment(input, 1)) => *input == value,
        Expression::Math(MathOperation::Add(left, right)) => {
            (*left == value
                && matches!(
                    definition(operations, *right),
                    Expression::Const(ConstantValue::Integer(1))
                ))
                || (*right == value
                    && matches!(
                        definition(operations, *left),
                        Expression::Const(ConstantValue::Integer(1))
                    ))
        }
        _ => false,
    }
}

#[test]
fn java_fixture_preserves_field_and_array_operands() {
    let ir = method_ir("IrSemantics", "run");
    let operations = operations(&ir);

    let read_value = operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Definition {
                value,
                expr: Expression::Array(ArrayOperation::Read { array_ref, index }),
            } => Some((*value, *array_ref, *index)),
            _ => None,
        })
        .expect("array read defines a value");
    let write = operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Effect {
                expr:
                    Expression::Array(ArrayOperation::Write {
                        array_ref,
                        index,
                        value,
                    }),
            } => Some((*array_ref, *index, *value)),
            _ => None,
        })
        .expect("array write is an effect");
    assert_eq!((write.0, write.1), (read_value.1, read_value.2));
    assert!(adds_one(
        &operations,
        definition(&operations, write.2),
        read_value.0
    ));
    assert!(operations.iter().any(|operation| matches!(
        operation,
        Operation::Definition { expr: Expression::Field(FieldAccess::ReadInstance { field, .. }), .. }
            if field.name == "value"
    )));
    assert!(operations.iter().any(|operation| matches!(
        operation,
        Operation::Effect { expr: Expression::Field(FieldAccess::WriteInstance { field, value, .. }) }
            if field.name == "value" && *value == read_value.0
    )));
    let dimensions = operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Definition {
                expr: Expression::Array(ArrayOperation::NewMultiDim { dimensions, .. }),
                ..
            } => Some(dimensions),
            _ => None,
        })
        .expect("multidimensional allocation defines a value");
    assert_eq!(dimensions.len(), 2);
    assert!(adds_one(
        &operations,
        definition(&operations, dimensions[0]),
        ir.parameters[1]
    ));
    assert!(matches!(
        definition(&operations, dimensions[1]),
        Expression::Const(ConstantValue::Integer(2))
    ));
}

#[test]
fn java_fixture_preserves_call_and_monitor_operands() {
    let ir = method_ir("IrSemantics", "run");
    let operations = operations(&ir);
    let interface_call = operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Definition {
                value,
                expr:
                    Expression::Call {
                        kind: InvocationKind::Interface { this },
                        method,
                        args,
                    },
            } if method.name == "applyAsInt" => Some((*value, *this, args)),
            _ => None,
        })
        .expect("interface call preserves its receiver and result");
    assert_eq!(interface_call.1, ir.parameters[3]);
    assert_eq!(interface_call.2.len(), 1);
    let static_call = operations
        .iter()
        .find_map(|operation| match operation {
            Operation::Definition {
                value,
                expr:
                    Expression::Call {
                        kind: InvocationKind::Static,
                        method,
                        args,
                    },
            } if method.name == "combine" => Some((*value, method, args)),
            _ => None,
        })
        .expect("static call defines its result");
    assert_eq!(static_call.1.descriptor, "(IJ)J".parse().unwrap());
    assert_eq!(static_call.2.len(), 2);
    assert!(matches!(
        definition(&operations, static_call.2[0]),
        Expression::Math(MathOperation::Add(_, _))
    ));
    assert!(matches!(
        definition(&operations, static_call.2[1]),
        Expression::Conversion(Conversion::Int2Long(value)) if *value == interface_call.0
    ));
    assert!(reachable_blocks(&ir).iter().any(|(_, block)| matches!(
        block.terminator,
        Terminator::Return { value: Some(value), .. } if value == static_call.0
    )));

    let acquire = operations.iter().find_map(|operation| match operation {
        Operation::Effect {
            expr: Expression::Synchronization(LockOperation::Acquire(value)),
        } => Some(value),
        _ => None,
    });
    let release = operations.iter().find_map(|operation| match operation {
        Operation::Effect {
            expr: Expression::Synchronization(LockOperation::Release(value)),
        } => Some(value),
        _ => None,
    });
    assert_eq!(acquire, Some(&ir.parameters[2]));
    assert_eq!(release, acquire);
}

#[test]
fn java_fixture_lifts_capturing_lambda_closures() {
    let ir = method_ir("TestAnalysis", "test");
    assert!(operations(&ir).iter().any(|operation| matches!(
        operation,
        Operation::Definition {
            expr: Expression::Closure { name, captures, .. },
            ..
        } if name == "applyAsInt" && captures.as_slice() == &ir.parameters[..1]
    )));
}
