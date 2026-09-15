//! JVM frame representation used while constructing the instruction graph.

mod error;
mod local_variables;
mod operand_stack;
mod value_category;

#[cfg(test)]
mod tests;

pub use error::Error as FrameError;
pub(crate) use operand_stack::StackOperation;
pub(crate) use value_category::ValueCategory;

use crate::types::method_descriptor::MethodDescriptor;
use local_variables::LocalVariables;
use operand_stack::OperandStack;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Position {
    Local(usize),
    Stack(usize),
}

type PairedSlotValues<'a, V> = Vec<(Option<&'a V>, Option<&'a V>)>;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Frame<V> {
    pub locals: LocalVariables<V>,
    pub stack: OperandStack<V>,
}

impl<V> Frame<V> {
    pub fn into_unwind_frame(mut self) -> Self {
        self.locals.clear_for_unwind();
        self.stack.clear();
        self
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &V> {
        self.locals.values().chain(self.stack.values())
    }

    pub fn handler_exception(&self) -> Result<&V, FrameError> {
        self.stack.single_value(ValueCategory::Category1)
    }

    pub fn paired_slot_values<'a>(
        &'a self,
        other: &'a Self,
    ) -> Result<PairedSlotValues<'a, V>, FrameError> {
        self.ensure_compatible_shape(other)?;
        Ok(self
            .locals
            .slot_values()
            .chain(self.stack.slot_values())
            .zip(other.locals.slot_values().chain(other.stack.slot_values()))
            .collect())
    }

    pub fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(Position, &mut V, V) -> bool,
    ) -> Result<bool, FrameError> {
        self.ensure_compatible_shape(&other)?;
        let locals_changed = self
            .locals
            .merge_from_with(other.locals, |index, lhs, rhs| {
                merge_values(Position::Local(index), lhs, rhs)
            });
        let stack_changed = self.stack.merge_from_with(other.stack, |index, lhs, rhs| {
            merge_values(Position::Stack(index), lhs, rhs)
        });
        Ok(locals_changed || stack_changed)
    }

    fn ensure_compatible_shape(&self, other: &Self) -> Result<(), FrameError> {
        if !self.locals.has_same_shape(&other.locals) || !self.stack.has_same_shape(&other.stack) {
            return Err(FrameError::IncompatibleFrameShape);
        }
        Ok(())
    }
}

impl<V: Clone> Frame<V> {
    pub fn for_method_entry(
        descriptor: &MethodDescriptor,
        max_locals: u16,
        max_operand_stack: u16,
        this_value: Option<V>,
        parameters: &[V],
    ) -> Result<Self, FrameError> {
        let local_variables =
            LocalVariables::for_method_entry(descriptor, max_locals, this_value, parameters)?;
        let operand_stack = OperandStack::with_max_slots(max_operand_stack);
        Ok(Self {
            locals: local_variables,
            stack: operand_stack,
        })
    }

    pub fn exception_handler_frame(&self, caught: V) -> Result<Self, FrameError> {
        let locals = self.locals.clone();
        let stack = OperandStack::with_max_slots(self.stack.max_slots());
        let mut frame = Self { locals, stack };
        frame.stack.push(caught, ValueCategory::Category1)?;
        Ok(frame)
    }
}
