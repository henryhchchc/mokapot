use std::iter::once;

use itertools::Itertools;

use crate::types::{
    field_type::{FieldType, PrimitiveType},
    method_descriptor::MethodDescriptor,
};

pub(crate) const CATEGORY_1: bool = false;
pub(crate) const CATEGORY_2: bool = true;

use crate::ir::generator::jvm::frame::{entry::Entry, error::JvmFrameError};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Position {
    Local(usize),
    Stack(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct Frame<V> {
    max_operand_stack: u16,
    local_slots: Box<[Entry<V>]>,
    operand_slots: Vec<Entry<V>>,
}

impl<V> Frame<V> {
    pub fn into_unwind_frame(mut self) -> Self {
        for entry in &mut self.local_slots {
            if matches!(entry, Entry::Value(_) | Entry::UnsetLocal) {
                *entry = Entry::UnsetLocal;
            }
        }
        self.operand_slots.clear();
        self
    }

    pub fn iter_values(&self) -> impl Iterator<Item = &V> {
        self.local_slots
            .iter()
            .chain(&self.operand_slots)
            .filter_map(|entry| match entry {
                Entry::Value(value) => Some(value),
                Entry::Top | Entry::UnsetLocal | Entry::Unavailable => None,
            })
    }

    pub fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(Position, &mut V, V) -> bool,
    ) -> bool {
        assert_eq!(self.max_operand_stack, other.max_operand_stack);
        let locals_changed = self
            .local_slots
            .iter_mut()
            .zip_eq(other.local_slots)
            .enumerate()
            .fold(false, |changed, (index, (lhs, rhs))| {
                lhs.merge_from_with(rhs, |lhs, rhs| {
                    merge_values(Position::Local(index), lhs, rhs)
                }) || changed
            });
        let stack_changed = self
            .operand_slots
            .iter_mut()
            .zip_eq(other.operand_slots)
            .enumerate()
            .fold(false, |changed, (index, (lhs, rhs))| {
                lhs.merge_from_with(rhs, |lhs, rhs| {
                    merge_values(Position::Stack(index), lhs, rhs)
                }) || changed
            });
        locals_changed || stack_changed
    }
}

impl<V: Clone> Frame<V> {
    pub fn for_method_entry(
        desc: &MethodDescriptor,
        max_locals: u16,
        max_operand_stack: u16,
        this_value: Option<V>,
        parameters: &[V],
    ) -> Result<Self, JvmFrameError> {
        if parameters.len() != desc.parameters_types.len() {
            return Err(JvmFrameError::ParameterCountMismatch);
        }
        let local_slots = create_local_slots(desc, max_locals, this_value, parameters)?;
        Ok(Self {
            max_operand_stack,
            local_slots,
            operand_slots: Vec::with_capacity(max_operand_stack.into()),
        })
    }

    pub fn pop_slot(&mut self) -> Result<Entry<V>, JvmFrameError> {
        self.operand_slots
            .pop()
            .ok_or(JvmFrameError::StackUnderflow)
    }

    pub fn push_slot(&mut self, value: Entry<V>) -> Result<(), JvmFrameError> {
        let stack_size =
            u16::try_from(self.operand_slots.len()).expect("The stack size should be within u16");
        if stack_size >= self.max_operand_stack {
            Err(JvmFrameError::StackOverflow)
        } else {
            self.operand_slots.push(value);
            Ok(())
        }
    }

    pub fn pop_value<const IS_CATEGORY_2: bool>(&mut self) -> Result<V, JvmFrameError> {
        let value = match self.pop_slot()? {
            Entry::Value(it) => Ok(it),
            Entry::Top => Err(JvmFrameError::InvalidSlotLayout),
            Entry::UnsetLocal | Entry::Unavailable => {
                unreachable!("It is never pushed to the stack")
            }
        }?;
        if IS_CATEGORY_2 {
            match self.pop_slot()? {
                Entry::Top => Ok(()),
                Entry::Value(_) => Err(JvmFrameError::InvalidSlotLayout),
                Entry::UnsetLocal | Entry::Unavailable => {
                    unreachable!("It is never pushed to the stack")
                }
            }?;
        }
        Ok(value)
    }

    pub fn push_value<const IS_CATEGORY_2: bool>(&mut self, value: V) -> Result<(), JvmFrameError> {
        if IS_CATEGORY_2 {
            self.push_slot(Entry::Top)?;
        }
        self.push_slot(Entry::Value(value))
    }

    pub fn pop_arguments(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<V>, JvmFrameError> {
        let mut args: Vec<_> = descriptor
            .parameters_types
            .iter()
            .rev()
            .map(|param_type| self.pop_value_of_type(param_type))
            .try_collect()?;
        args.reverse();
        Ok(args)
    }

    pub fn push_value_of_type(
        &mut self,
        value_type: &FieldType,
        value: V,
    ) -> Result<(), JvmFrameError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.push_value::<CATEGORY_2>(value)
        } else {
            self.push_value::<CATEGORY_1>(value)
        }
    }

    pub fn pop_value_of_type(&mut self, value_type: &FieldType) -> Result<V, JvmFrameError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.pop_value::<CATEGORY_2>()
        } else {
            self.pop_value::<CATEGORY_1>()
        }
    }

    pub fn get_local<const IS_CATEGORY_2: bool>(&self, idx: u16) -> Result<V, JvmFrameError> {
        let idx = usize::from(idx);
        let lower_slot = self
            .local_slots
            .get(idx)
            .ok_or(JvmFrameError::LocalIndexOutOfBounds)?;
        let value = match lower_slot {
            Entry::Value(it) => Ok(it.clone()),
            Entry::Top => Err(JvmFrameError::InvalidSlotLayout),
            Entry::Unavailable => Err(JvmFrameError::UnavailableLocal),
            Entry::UnsetLocal => Err(JvmFrameError::UninitializedLocal),
        }?;
        if IS_CATEGORY_2 {
            let higher_slot = self
                .local_slots
                .get(idx + 1)
                .ok_or(JvmFrameError::LocalIndexOutOfBounds)?;
            match higher_slot {
                Entry::Top => Ok(()),
                _ => Err(JvmFrameError::InvalidSlotLayout),
            }?;
        }

        Ok(value)
    }

    pub fn set_local<const IS_CATEGORY_2: bool>(
        &mut self,
        idx: u16,
        value: V,
    ) -> Result<(), JvmFrameError> {
        let idx = usize::from(idx);
        let lower_slot = self
            .local_slots
            .get_mut(idx)
            .ok_or(JvmFrameError::LocalIndexOutOfBounds)?;
        *lower_slot = Entry::Value(value);

        if IS_CATEGORY_2 {
            let higher_slot = self
                .local_slots
                .get_mut(idx + 1)
                .ok_or(JvmFrameError::LocalIndexOutOfBounds)?;
            *higher_slot = Entry::Top;
        }

        Ok(())
    }

    pub fn with_single_stack_entry(&self, stack_value: Entry<V>) -> Self {
        let mut operand_slots = Vec::with_capacity(self.max_operand_stack.into());

        operand_slots.push(stack_value);
        Self {
            max_operand_stack: self.max_operand_stack,
            local_slots: self.local_slots.clone(),
            operand_slots,
        }
    }

    pub fn with_empty_operand_stack(&self) -> Self {
        Self {
            max_operand_stack: self.max_operand_stack,
            local_slots: self.local_slots.clone(),
            operand_slots: Vec::with_capacity(self.max_operand_stack.into()),
        }
    }

    pub fn local_slots(&self) -> &[Entry<V>] {
        &self.local_slots
    }

    pub fn operand_slots(&self) -> &[Entry<V>] {
        &self.operand_slots
    }
}

fn create_local_slots<V: Clone>(
    desc: &MethodDescriptor,
    max_locals: u16,
    this_value: Option<V>,
    parameters: &[V],
) -> Result<Box<[Entry<V>]>, JvmFrameError> {
    use PrimitiveType::{Double, Long};
    let locals_for_args = desc
        .parameters_types
        .iter()
        .map(|it| match it {
            FieldType::Base(Long | Double) => 2,
            _ => 1,
        })
        .sum::<usize>()
        + usize::from(this_value.is_some());
    if usize::from(max_locals) < locals_for_args {
        return Err(JvmFrameError::LocalIndexOutOfBounds);
    }
    let this_arg = this_value.map(Entry::Value);
    let args = desc
        .parameters_types
        .iter()
        .zip(parameters.iter().cloned())
        .flat_map(|(local_type, value)| {
            let maybe_top = if let FieldType::Base(Long | Double) = local_type {
                Some(Entry::Top)
            } else {
                None
            };
            once(Entry::Value(value)).chain(maybe_top)
        });
    let local_slots = this_arg
        .into_iter()
        .chain(args)
        .pad_using(max_locals.into(), |_| Entry::UnsetLocal)
        .collect();
    Ok(local_slots)
}
