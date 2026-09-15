use crate::{
    ir::{
        expression::Expression,
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::ValueCategory::{self, Category1},
                symbolic_execution::{RegisterInstruction, Value, lifting::Context},
            },
        },
    },
    jvm::references::MethodRef,
    types::method_descriptor::{MethodDescriptor, ReturnType},
};

impl Context<'_, '_, '_> {
    pub(super) fn invoke(
        &mut self,
        method: &MethodRef,
        has_receiver: bool,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let definition = self.definition_id_for_return(&method.descriptor.return_type)?;
        let args = self.frame.stack.pop_arguments(&method.descriptor)?;
        let this = has_receiver
            .then(|| self.frame.stack.pop(Category1))
            .transpose()?;
        let expr = Expression::Call {
            method: method.clone(),
            this,
            args,
        };
        self.finish_call(&method.descriptor, definition, expr)
    }

    pub(super) fn invoke_dynamic(
        &mut self,
        descriptor: &MethodDescriptor,
        bootstrap_method_index: u16,
        name: &str,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let definition = self.definition_id_for_return(&descriptor.return_type)?;
        let expr = Expression::Closure {
            captures: self.frame.stack.pop_arguments(descriptor)?,
            bootstrap_method_index,
            name: name.to_owned(),
            closure_descriptor: descriptor.clone(),
        };
        self.finish_call(descriptor, definition, expr)
    }

    fn finish_call(
        &mut self,
        descriptor: &MethodDescriptor,
        definition: Option<SsaValueId>,
        expr: Expression<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        match &descriptor.return_type {
            ReturnType::Some(return_type) => {
                let value = definition.ok_or(MokaIRBuildError::MalformedControlFlow)?;
                self.frame
                    .stack
                    .push(value.into(), ValueCategory::of_field_type(return_type))?;
                Ok(RegisterInstruction::Definition { value, expr })
            }
            ReturnType::Void => Ok(RegisterInstruction::Effect(expr)),
        }
    }
}
