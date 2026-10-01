use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Expression, InvocationKind},
    },
    jvm::references::MethodRef,
    types::{
        field_type::ValueCategory,
        method_descriptor::{MethodDescriptor, ReturnType},
    },
};

#[derive(Clone, Copy)]
enum CallResult {
    Value(ValueId, ValueCategory),
    Void,
}

impl LiftContext<'_, '_> {
    pub fn invoke_static(&mut self, method: &MethodRef) -> Result<Option<Operation>, FrameError> {
        let result = self.call_result(&method.descriptor.return_type);
        let expr = Expression::Call {
            kind: InvocationKind::Static,
            method: method.clone(),
            args: self.frame.stack.pop_arguments(&method.descriptor)?,
        };
        self.finish_call(result, expr)
    }

    pub fn invoke_instance(
        &mut self,
        method: &MethodRef,
        kind: impl FnOnce(ValueId) -> InvocationKind,
    ) -> Result<Option<Operation>, FrameError> {
        let result = self.call_result(&method.descriptor.return_type);
        let args = self.frame.stack.pop_arguments(&method.descriptor)?;
        let this = self.frame.stack.pop(Category1)?;
        let expr = Expression::Call {
            kind: kind(this),
            method: method.clone(),
            args,
        };
        self.finish_call(result, expr)
    }

    pub fn invoke_dynamic(
        &mut self,
        descriptor: &MethodDescriptor,
        bootstrap_method_index: u16,
        name: &str,
    ) -> Result<Option<Operation>, FrameError> {
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
    ) -> Result<Option<Operation>, FrameError> {
        match result {
            CallResult::Value(value, category) => {
                self.frame.stack.push(value, category)?;
                Ok(Some(Operation::Definition { value, expr }))
            }
            CallResult::Void => Ok(Some(Operation::Effect { expr })),
        }
    }

    fn call_result(&mut self, return_type: &ReturnType) -> CallResult {
        match return_type {
            ReturnType::Some(return_type) => {
                CallResult::Value(self.definition_id(), return_type.value_category())
            }
            ReturnType::Void => CallResult::Void,
        }
    }
}
