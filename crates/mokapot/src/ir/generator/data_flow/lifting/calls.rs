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

impl<'class> LiftContext<'_, '_> {
    pub fn invoke_static(
        &mut self,
        method: MethodRef<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let descriptor = method.descriptor();
        let expr = Expression::Call {
            kind: InvocationKind::Static,
            method,
            args: self.frame.stack.pop_arguments(descriptor)?,
        };
        self.finish_call(&descriptor.return_type, expr)
    }

    pub fn invoke_instance(
        &mut self,
        method: MethodRef<'class>,
        kind: impl FnOnce(ValueId) -> InvocationKind,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let descriptor = method.descriptor();
        let args = self.frame.stack.pop_arguments(descriptor)?;
        let this = self.frame.stack.pop(Category1)?;
        let expr = Expression::Call {
            kind: kind(this),
            method,
            args,
        };
        self.finish_call(&descriptor.return_type, expr)
    }

    pub fn invoke_dynamic(
        &mut self,
        descriptor: &'class MethodDescriptor,
        bootstrap_method_index: u16,
        name: &'class JavaStr,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let expr = Expression::Closure {
            captures: self.frame.stack.pop_arguments(descriptor)?,
            bootstrap_method_index,
            name,
            closure_descriptor: descriptor,
        };
        self.finish_call(&descriptor.return_type, expr)
    }

    fn finish_call(
        &mut self,
        return_type: &ReturnType,
        expr: Expression<'class>,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        match return_type {
            ReturnType::Some(return_type) => {
                self.define_on_stack(return_type.value_category(), expr)
            }
            ReturnType::Void => Ok(Some(Operation::Effect(expr))),
        }
    }
}
