use crate::{
    ir::{
        expression::Expression,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, Frame},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::Value,
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::method_descriptor::ReturnType,
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    match instruction {
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

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    let instruction = match jvm_instruction {
        JVM::InvokeVirtual(method_ref)
        | JVM::InvokeSpecial(method_ref)
        | JVM::InvokeInterface(method_ref, _) => {
            let arguments = frame.pop_arguments(&method_ref.descriptor)?;
            let object_ref = frame.pop_value::<CATEGORY_1>()?;
            let expression = Expression::Call {
                method: method_ref.clone(),
                this: Some(object_ref),
                args: arguments,
            };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let definition = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, definition.into())?;
                    RegisterInstruction::Definition {
                        value: definition,
                        expr: expression,
                    }
                }
                ReturnType::Void => RegisterInstruction::Effect(expression),
            }
        }
        JVM::InvokeStatic(method_ref) => {
            let arguments = frame.pop_arguments(&method_ref.descriptor)?;
            let expression = Expression::Call {
                method: method_ref.clone(),
                this: None,
                args: arguments,
            };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let definition = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, definition.into())?;
                    RegisterInstruction::Definition {
                        value: definition,
                        expr: expression,
                    }
                }
                ReturnType::Void => RegisterInstruction::Effect(expression),
            }
        }
        JVM::InvokeDynamic {
            descriptor,
            bootstrap_method_index,
            name,
        } => {
            let arguments = frame.pop_arguments(descriptor)?;
            let expression = Expression::Closure {
                bootstrap_method_index: *bootstrap_method_index,
                name: name.to_owned(),
                captures: arguments,
                closure_descriptor: descriptor.to_owned(),
            };
            match &descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let definition = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, definition.into())?;
                    RegisterInstruction::Definition {
                        value: definition,
                        expr: expression,
                    }
                }
                ReturnType::Void => RegisterInstruction::Effect(expression),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
