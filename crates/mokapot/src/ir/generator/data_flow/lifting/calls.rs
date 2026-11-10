use ValueCategory::Category1;

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation, ValueId,
        expression::{Expression, InvocationKind},
    },
    jvm::{JavaStr, references::MethodRef},
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

impl<'class> LiftContext<'_, '_> {
    pub fn invoke_static(
        &mut self,
        method: MethodRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let result = self.call_result(&method.descriptor().return_type);
        let args = self.frame.stack.pop_arguments(method.descriptor())?;
        self.finish_call(
            result,
            Expression::Call {
                kind: InvocationKind::Static,
                method,
                args,
            },
        )
    }

    pub fn invoke_instance(
        &mut self,
        method: MethodRef<'class>,
        kind: impl FnOnce(ValueId) -> InvocationKind,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let result = self.call_result(&method.descriptor().return_type);
        let args = self.frame.stack.pop_arguments(method.descriptor())?;
        let this = self.frame.stack.pop(Category1)?;
        self.finish_call(
            result,
            Expression::Call {
                kind: kind(this),
                method,
                args,
            },
        )
    }

    pub fn invoke_dynamic(
        &mut self,
        descriptor: &'class MethodDescriptor,
        bootstrap_method_index: u16,
        name: &'class JavaStr,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let result = self.call_result(&descriptor.return_type);
        let captures = self.frame.stack.pop_arguments(descriptor)?;
        self.finish_call(
            result,
            Expression::Closure {
                captures,
                bootstrap_method_index,
                name,
                closure_descriptor: descriptor,
            },
        )
    }

    fn finish_call(
        &mut self,
        result: CallResult,
        expr: Expression<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        if let CallResult::Value(value, category) = result {
            self.frame.stack.push(value, category)?;
        }
        Ok(Some(match result {
            CallResult::Value(value, _) => Operation::Definition { value, expr },
            CallResult::Void => Operation::Effect { expr },
        }))
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
