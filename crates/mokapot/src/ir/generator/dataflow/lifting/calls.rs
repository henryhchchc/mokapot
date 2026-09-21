use ValueCategory::Category1;

use super::{LiftContext, ValueCategory, definition_operation};
use crate::{
    ir::{Operation, ValueId, expression::Expression, generator::error::Error},
    jvm::references::MethodRef,
    types::method_descriptor::{MethodDescriptor, ReturnType},
};

#[derive(Clone, Copy)]
enum CallResult {
    Value(ValueId, ValueCategory),
    Void,
}

impl LiftContext<'_, '_> {
    pub(super) fn invoke(
        &mut self,
        method: &MethodRef,
        has_receiver: bool,
    ) -> Result<Option<Operation>, Error> {
        let result = self.call_result(&method.descriptor.return_type);
        let args = self.frame.stack.pop_arguments(&method.descriptor)?;
        let this = has_receiver
            .then(|| self.frame.stack.pop(Category1))
            .transpose()?;
        let expr = Expression::Call {
            method: method.clone(),
            this,
            args,
        };
        self.finish_call(result, expr)
    }

    pub(super) fn invoke_dynamic(
        &mut self,
        descriptor: &MethodDescriptor,
        bootstrap_method_index: u16,
        name: &str,
    ) -> Result<Option<Operation>, Error> {
        let result = self.call_result(&descriptor.return_type);
        let expr = Expression::Closure {
            captures: self.frame.stack.pop_arguments(descriptor)?,
            bootstrap_method_index,
            name: name.to_owned(),
            closure_descriptor: descriptor.clone(),
        };
        self.finish_call(result, expr)
    }

    fn finish_call(
        &mut self,
        result: CallResult,
        expr: Expression,
    ) -> Result<Option<Operation>, Error> {
        match result {
            CallResult::Value(value, category) => {
                self.frame.stack.push(value, category)?;
                Ok(Some(definition_operation(value, expr)))
            }
            CallResult::Void => Ok(Some(Operation::Effect { expr })),
        }
    }

    fn call_result(&mut self, return_type: &ReturnType) -> CallResult {
        match return_type {
            ReturnType::Some(return_type) => CallResult::Value(
                self.definition_id(),
                ValueCategory::of_field_type(return_type),
            ),
            ReturnType::Void => CallResult::Void,
        }
    }
}
