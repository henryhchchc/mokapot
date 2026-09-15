mod error;
mod local_variables;
mod operand_stack;
mod value_category;

#[cfg(test)]
mod tests;

pub use error::JvmFrameError;
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
    pub local_variables: LocalVariables<V>,
    pub operand_stack: OperandStack<V>,
}

impl<V> Frame<V> {
    pub fn into_unwind_frame(mut self) -> Self {
        self.local_variables.clear_for_unwind();
        self.operand_stack.clear();
        self
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &V> {
        self.local_variables
            .values()
            .chain(self.operand_stack.values())
    }

    pub fn paired_slot_values<'a>(
        &'a self,
        other: &'a Self,
    ) -> Result<PairedSlotValues<'a, V>, JvmFrameError> {
        self.ensure_compatible_shape(other)?;
        Ok(self
            .local_variables
            .slot_values()
            .chain(self.operand_stack.slot_values())
            .zip(
                other
                    .local_variables
                    .slot_values()
                    .chain(other.operand_stack.slot_values()),
            )
            .collect())
    }

    pub fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(Position, &mut V, V) -> bool,
    ) -> Result<bool, JvmFrameError> {
        self.ensure_compatible_shape(&other)?;
        let locals_changed = self
            .local_variables
            .merge_from_with(other.local_variables, |index, lhs, rhs| {
                merge_values(Position::Local(index), lhs, rhs)
            });
        let stack_changed = self
            .operand_stack
            .merge_from_with(other.operand_stack, |index, lhs, rhs| {
                merge_values(Position::Stack(index), lhs, rhs)
            });
        Ok(locals_changed || stack_changed)
    }

    fn ensure_compatible_shape(&self, other: &Self) -> Result<(), JvmFrameError> {
        if !self.local_variables.has_same_shape(&other.local_variables)
            || !self.operand_stack.has_same_shape(&other.operand_stack)
        {
            return Err(JvmFrameError::IncompatibleFrameShape);
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
    ) -> Result<Self, JvmFrameError> {
        Ok(Self {
            local_variables: LocalVariables::for_method_entry(
                descriptor, max_locals, this_value, parameters,
            )?,
            operand_stack: OperandStack::with_max_slots(max_operand_stack),
        })
    }

    pub fn exception_handler_frame(&self, caught: V) -> Result<Self, JvmFrameError> {
        let mut frame = Self {
            local_variables: self.local_variables.clone(),
            operand_stack: OperandStack::with_max_slots(self.operand_stack.max_slots()),
        };
        frame.operand_stack.push(caught, ValueCategory::Category1)?;
        Ok(frame)
    }
}
