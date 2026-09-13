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
            let args = frame.pop_arguments(&method_ref.descriptor)?;
            let this = Some(frame.pop_value::<CATEGORY_1>()?);
            let method = method_ref.clone();
            let expr = Expression::Call { method, this, args };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let value = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, value.into())?;
                    RegisterInstruction::Definition { value, expr }
                }
                ReturnType::Void => RegisterInstruction::Effect(expr),
            }
        }
        JVM::InvokeStatic(method_ref) => {
            let args = frame.pop_arguments(&method_ref.descriptor)?;
            let expr = Expression::Call {
                method: method_ref.clone(),
                this: None,
                args,
            };
            match &method_ref.descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let value = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, value.into())?;
                    RegisterInstruction::Definition { value, expr }
                }
                ReturnType::Void => RegisterInstruction::Effect(expr),
            }
        }
        JVM::InvokeDynamic {
            descriptor,
            bootstrap_method_index,
            name,
        } => {
            let expr = Expression::Closure {
                captures: frame.pop_arguments(descriptor)?,
                bootstrap_method_index: *bootstrap_method_index,
                name: name.to_owned(),
                closure_descriptor: descriptor.to_owned(),
            };
            match &descriptor.return_type {
                ReturnType::Some(return_type) => {
                    let value = require_definition_id(definition)?;
                    frame.push_value_of_type(return_type, value.into())?;
                    RegisterInstruction::Definition { value, expr }
                }
                ReturnType::Void => RegisterInstruction::Effect(expr),
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
