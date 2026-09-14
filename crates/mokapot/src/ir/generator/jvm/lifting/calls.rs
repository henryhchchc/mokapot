use crate::{
    ir::{
        expression::Expression,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, Frame},
                instruction::RegisterInstruction,
                subroutine_expansion::Location,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::method_descriptor::ReturnType,
};

impl Executor<'_> {
    pub(super) fn try_lift_calls(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        frame: &mut Frame<Value>,
    ) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
        let instruction = match jvm_instruction {
            JVM::InvokeVirtual(method_ref)
            | JVM::InvokeSpecial(method_ref)
            | JVM::InvokeInterface(method_ref, _) => {
                let definition =
                    self.definition_id_for_return(location, &method_ref.descriptor.return_type)?;
                let args = frame.pop_arguments(&method_ref.descriptor)?;
                let this = Some(frame.pop_value::<CATEGORY_1>()?);
                let method = method_ref.clone();
                let expr = Expression::Call { method, this, args };
                match &method_ref.descriptor.return_type {
                    ReturnType::Some(return_type) => {
                        let value = definition.ok_or(MokaIRBuildError::MalformedControlFlow)?;
                        frame.push_value_of_type(return_type, value.into())?;
                        RegisterInstruction::Definition { value, expr }
                    }
                    ReturnType::Void => RegisterInstruction::Effect(expr),
                }
            }
            JVM::InvokeStatic(method_ref) => {
                let definition =
                    self.definition_id_for_return(location, &method_ref.descriptor.return_type)?;
                let args = frame.pop_arguments(&method_ref.descriptor)?;
                let expr = Expression::Call {
                    method: method_ref.clone(),
                    this: None,
                    args,
                };
                match &method_ref.descriptor.return_type {
                    ReturnType::Some(return_type) => {
                        let value = definition.ok_or(MokaIRBuildError::MalformedControlFlow)?;
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
                let definition =
                    self.definition_id_for_return(location, &descriptor.return_type)?;
                let expr = Expression::Closure {
                    captures: frame.pop_arguments(descriptor)?,
                    bootstrap_method_index: *bootstrap_method_index,
                    name: name.to_owned(),
                    closure_descriptor: descriptor.to_owned(),
                };
                match &descriptor.return_type {
                    ReturnType::Some(return_type) => {
                        let value = definition.ok_or(MokaIRBuildError::MalformedControlFlow)?;
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

    fn definition_id_for_return(
        &mut self,
        location: Location,
        return_type: &ReturnType,
    ) -> Result<Option<SsaValueId>, MokaIRBuildError> {
        match return_type {
            ReturnType::Some(_) => self.definition_id_at(location).map(Some),
            ReturnType::Void => Ok(None),
        }
    }
}
