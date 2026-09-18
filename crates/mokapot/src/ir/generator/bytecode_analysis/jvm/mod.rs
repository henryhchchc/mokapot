//! JVM frame representation used while analyzing structural blocks.

mod error;
mod local_variables;
mod operand_stack;
mod value_category;

pub use error::Error as FrameError;
pub(crate) use local_variables::EntrySlots;
pub(crate) use operand_stack::StackOperation;
pub(crate) use value_category::ValueCategory;

use crate::{ir::ValueId, types::method_descriptor::MethodDescriptor};
use local_variables::LocalVariables;
use operand_stack::OperandStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Position {
    Local(usize),
    Stack(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Frame {
    pub locals: LocalVariables,
    pub stack: OperandStack,
}

impl Frame {
    pub fn into_unwind_frame(mut self) -> Self {
        self.locals.clear_for_unwind();
        self.stack.clear();
        self
    }

    pub fn value_at(&self, position: Position) -> Option<&ValueId> {
        match position {
            Position::Local(index) => self.locals.slot_values().nth(index).flatten(),
            Position::Stack(index) => self.stack.slot_values().nth(index).flatten(),
        }
    }

    pub fn handler_exception(&self) -> Result<&ValueId, FrameError> {
        self.stack.single_value(ValueCategory::Category1)
    }

    pub fn merge_from_with<E>(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(Position, &mut ValueId, ValueId) -> Result<(), E>,
    ) -> Result<(), E>
    where
        E: From<FrameError>,
    {
        self.ensure_compatible_shape(&other).map_err(E::from)?;
        self.locals
            .merge_from_with(other.locals, |index, lhs, rhs| {
                merge_values(Position::Local(index), lhs, rhs)
            })?;
        self.stack.merge_from_with(other.stack, |index, lhs, rhs| {
            merge_values(Position::Stack(index), lhs, rhs)
        })
    }

    fn ensure_compatible_shape(&self, other: &Self) -> Result<(), FrameError> {
        if !self.locals.has_same_shape(&other.locals) || !self.stack.has_same_shape(&other.stack) {
            return Err(FrameError::IncompatibleFrameShape);
        }
        Ok(())
    }
    /// Builds the entry frame of a method, also reporting the slots it assigned
    /// to the receiver and to the parameters.
    ///
    /// The parameters follow the receiver in descriptor order, with a category-2
    /// parameter occupying two slots; the callers of this constructor rely on
    /// that convention to map parameter identities to slots.
    pub fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_locals: u16,
        max_operand_stack: u16,
        this_value: Option<ValueId>,
        parameters: &[ValueId],
    ) -> Result<(Self, EntrySlots), FrameError> {
        let (locals, entry_slots) =
            LocalVariables::for_method_entry(descriptor, max_locals, this_value, parameters)?;
        let operand_stack = OperandStack::with_max_slots(max_operand_stack);
        Ok((
            Self {
                locals,
                stack: operand_stack,
            },
            entry_slots,
        ))
    }

    pub fn exception_handler_frame(&self, caught: ValueId) -> Result<Self, FrameError> {
        let locals = self.locals.clone();
        let stack = OperandStack::with_max_slots(self.stack.max_slots());
        let mut frame = Self { locals, stack };
        frame.stack.push(caught, ValueCategory::Category1)?;
        Ok(frame)
    }
}
