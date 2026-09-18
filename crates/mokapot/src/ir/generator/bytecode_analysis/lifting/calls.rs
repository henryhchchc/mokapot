use super::definition_operation;
use crate::{
    ir::{
        OperationKind, ValueId,
        expression::Expression,
        generator::{
            bytecode_analysis::{
                jvm::ValueCategory::{self, Category1},
                lifting::Context,
            },
            error::Error,
        },
    },
    jvm::references::MethodRef,
    types::method_descriptor::{MethodDescriptor, ReturnType},
};

impl Context<'_, '_> {
    pub(super) fn invoke(
        &mut self,
        method: &MethodRef,
        has_receiver: bool,
    ) -> Result<Option<OperationKind>, Error> {
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
    ) -> Result<Option<OperationKind>, Error> {
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
        definition: Option<ValueId>,
        expr: Expression,
    ) -> Result<Option<OperationKind>, Error> {
        match &descriptor.return_type {
            ReturnType::Some(return_type) => {
                let value = definition
                    .ok_or_else(|| Error::internal("a non-void call has no result identity"))?;
                self.frame
                    .stack
                    .push(value, ValueCategory::of_field_type(return_type))?;
                Ok(Some(definition_operation(value, expr)))
            }
            ReturnType::Void => Ok(Some(OperationKind::Effect { expr })),
        }
    }
}
