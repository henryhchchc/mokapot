use super::{
    DUAL_SLOT, Expression, FieldAccess, FieldType, FrameOperand, Instruction, JVM, JvmStackFrame,
    MokaIRBuildError, PrimitiveType, ReturnType, SINGLE_SLOT, SsaValueId, required_definition,
};

pub(super) const fn defines_value(instruction: &JVM) -> bool {
    match instruction {
        JVM::GetStatic(_) | JVM::GetField(_) => true,
        JVM::InvokeVirtual(method)
        | JVM::InvokeSpecial(method)
        | JVM::InvokeInterface(method, _)
        | JVM::InvokeStatic(method) => !matches!(method.descriptor.return_type, ReturnType::Void),
        JVM::InvokeDynamic { descriptor, .. } => {
            !matches!(descriptor.return_type, ReturnType::Void)
        }
        _ => false,
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
pub(super) fn lift<OP: FrameOperand>(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<Instruction<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        GetStatic(field) => {
            let def = required_definition(definition)?;
            frame.typed_push(&field.field_type, def.into())?;
            let field = field.clone();
            let field_op = FieldAccess::ReadStatic { field };
            Instruction::Definition {
                value: def,
                expr: Expression::Field(field_op),
            }
        }
        GetField(field) => {
            let def = required_definition(definition)?;
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let field = field.clone();
            frame.typed_push(&field.field_type, def.into())?;
            let field_op = FieldAccess::ReadInstance { object_ref, field };
            Instruction::Definition {
                value: def,
                expr: Expression::Field(field_op),
            }
        }
        PutStatic(field) => {
            use PrimitiveType::{Double, Long};
            let value = if let FieldType::Base(Double | Long) = field.field_type {
                frame.pop_value::<DUAL_SLOT>()
            } else {
                frame.pop_value::<SINGLE_SLOT>()
            }?;
            let field_op = FieldAccess::WriteStatic {
                field: field.clone(),
                value,
            };
            Instruction::Effect(Expression::Field(field_op))
        }
        PutField(field) => {
            use PrimitiveType::{Double, Long};
            let value = if let FieldType::Base(Double | Long) = field.field_type {
                frame.pop_value::<DUAL_SLOT>()
            } else {
                frame.pop_value::<SINGLE_SLOT>()
            }?;
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let field_op = FieldAccess::WriteInstance {
                object_ref,
                field: field.clone(),
                value,
            };
            Instruction::Effect(Expression::Field(field_op))
        }
        InvokeVirtual(method_ref) | InvokeSpecial(method_ref) | InvokeInterface(method_ref, _) => {
            let arguments = frame.pop_args(&method_ref.descriptor)?;
            let object_ref = frame.pop_value::<SINGLE_SLOT>()?;
            let rhs = Expression::Call {
                method: method_ref.clone(),
                this: Some(object_ref),
                args: arguments,
            };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let def = required_definition(definition)?;
                    frame.typed_push(return_type, def.into())?;
                    Instruction::Definition {
                        value: def,
                        expr: rhs,
                    }
                }
                ReturnType::Void => Instruction::Effect(rhs),
            }
        }
        InvokeStatic(method_ref) => {
            let arguments = frame.pop_args(&method_ref.descriptor)?;
            let rhs = Expression::Call {
                method: method_ref.clone(),
                this: None,
                args: arguments,
            };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let def = required_definition(definition)?;
                    frame.typed_push(return_type, def.into())?;
                    Instruction::Definition {
                        value: def,
                        expr: rhs,
                    }
                }
                ReturnType::Void => Instruction::Effect(rhs),
            }
        }
        InvokeDynamic {
            descriptor,
            bootstrap_method_index,
            name,
        } => {
            let arguments = frame.pop_args(descriptor)?;
            let rhs = Expression::Closure {
                bootstrap_method_index: *bootstrap_method_index,
                name: name.to_owned(),
                captures: arguments,
                closure_descriptor: descriptor.to_owned(),
            };
            match &descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let def = required_definition(definition)?;
                    frame.typed_push(return_type, def.into())?;
                    Instruction::Definition {
                        value: def,
                        expr: rhs,
                    }
                }
                ReturnType::Void => Instruction::Effect(rhs),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
