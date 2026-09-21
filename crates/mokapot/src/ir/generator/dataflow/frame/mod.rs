//! JVM frame representation used while analyzing structural blocks.

mod error;
mod local_variables;
mod operand_stack;
mod value_category;

#[cfg(test)]
mod tests;

pub use error::Error as FrameError;
use local_variables::LocalVariables;
use operand_stack::OperandStack;
pub(super) use operand_stack::StackOperation;
pub(super) use value_category::ValueCategory;

use crate::{ir::ValueId, types::method_descriptor::MethodDescriptor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum Position {
    Local(usize),
    Stack(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(super) struct Frame {
    pub locals: LocalVariables,
    pub stack: OperandStack,
}

impl Frame {
    pub(super) fn value_at(&self, position: Position) -> Option<&ValueId> {
        match position {
            Position::Local(index) => self.locals.slot_values().nth(index).flatten(),
            Position::Stack(index) => self.stack.slot_values().nth(index).flatten(),
        }
    }

    pub(super) fn handler_exception(&self) -> Result<&ValueId, FrameError> {
        self.stack.single_value(ValueCategory::Category1)
    }

    pub(super) fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(Position, &mut ValueId, ValueId),
    ) -> Result<(), FrameError> {
        self.ensure_compatible_shape(&other)?;
        self.locals
            .merge_from_with(other.locals, |index, lhs, rhs| {
                merge_values(Position::Local(index), lhs, rhs);
            });
        self.stack.merge_from_with(other.stack, |index, lhs, rhs| {
            merge_values(Position::Stack(index), lhs, rhs);
        });
        Ok(())
    }

    fn ensure_compatible_shape(&self, other: &Self) -> Result<(), FrameError> {
        if !self.locals.has_same_shape(&other.locals) || !self.stack.has_same_shape(&other.stack) {
            return Err(FrameError::IncompatibleFrameShape);
        }
        Ok(())
    }

    pub(super) fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_locals: u16,
        max_operand_stack: u16,
        this_value: Option<ValueId>,
        parameters: &[ValueId],
    ) -> Result<Self, FrameError> {
        let locals =
            LocalVariables::for_method_entry(descriptor, max_locals, this_value, parameters)?;
        let stack = OperandStack::with_max_slots(max_operand_stack);
        Ok(Self { locals, stack })
    }

    pub(super) fn exception_handler_frame(&self, caught: ValueId) -> Result<Self, FrameError> {
        let locals = self.locals.clone();
        let stack = OperandStack::with_max_slots(self.stack.max_slots());
        let mut frame = Self { locals, stack };
        frame.stack.push(caught, ValueCategory::Category1)?;
        Ok(frame)
    }
}
