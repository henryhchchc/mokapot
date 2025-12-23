use std::{collections::BTreeSet, iter::once};

use itertools::Itertools;

use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::{Identifier, Operand},
    jvm::code::ProgramCounter,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};

pub(crate) type SlotWidth = bool;
pub(crate) const SINGLE_SLOT: SlotWidth = false;
pub(crate) const DUAL_SLOT: SlotWidth = true;

use super::{entry::Entry, error::ExecutionError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmStackFrame {
    max_stack: u16,
    local_variables: Box<[Entry]>,
    operand_stack: Vec<Entry>,
    pub possible_ret_addresses: BTreeSet<ProgramCounter>,
}

impl PartialOrd for JvmStackFrame {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self.max_stack != other.max_stack {
            return None;
        }
        todo!()
    }
}

impl JvmStackFrame {
    pub(crate) fn new(
        is_static: bool,
        desc: &MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
    ) -> Result<Self, ExecutionError> {
        let local_variables = create_local_variable_entries(is_static, desc, max_locals)?;
        Ok(Self {
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack.into()),
            possible_ret_addresses: BTreeSet::new(),
        })
    }

    pub(crate) fn pop_raw(&mut self) -> Result<Entry, ExecutionError> {
        self.operand_stack
            .pop()
            .ok_or(ExecutionError::StackUnderflow)
    }

    pub(crate) fn push_raw(&mut self, value: Entry) -> Result<(), ExecutionError> {
        let stack_size =
            u16::try_from(self.operand_stack.len()).expect("The stack size should be within u16");
        if stack_size >= self.max_stack {
            Err(ExecutionError::StackOverflow)
        } else {
            self.operand_stack.push(value);
            Ok(())
        }
    }

    pub(crate) fn pop_value<const SLOT: SlotWidth>(&mut self) -> Result<Operand, ExecutionError> {
        let value = match self.pop_raw()? {
            Entry::Value(it) => Ok(it),
            Entry::Top => Err(ExecutionError::ValueMismatch),
            Entry::UninitializedLocal => unreachable!("It is never pushed to the stack"),
        }?;
        if SLOT == DUAL_SLOT {
            match self.pop_raw()? {
                Entry::Top => Ok(()),
                Entry::Value(_) => Err(ExecutionError::ValueMismatch),
                Entry::UninitializedLocal => unreachable!("It is never pushed to the stack"),
            }?;
        }
        Ok(value)
    }

    pub(crate) fn push_value<const SLOT: SlotWidth>(
        &mut self,
        value: Operand,
    ) -> Result<(), ExecutionError> {
        if SLOT == DUAL_SLOT {
            self.push_raw(Entry::Top)?;
        }
        self.push_raw(Entry::Value(value))
    }

    pub(crate) fn pop_args(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<Operand>, ExecutionError> {
        let mut args: Vec<_> = descriptor
            .parameters_types
            .iter()
            .rev()
            .map(|param_type| self.typed_pop(param_type))
            .try_collect()?;
        args.reverse();
        Ok(args)
    }

    pub(crate) fn typed_push(
        &mut self,
        value_type: &FieldType,
        value: Operand,
    ) -> Result<(), ExecutionError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.push_value::<DUAL_SLOT>(value)
        } else {
            self.push_value::<SINGLE_SLOT>(value)
        }
    }

    pub(crate) fn typed_pop(&mut self, value_type: &FieldType) -> Result<Operand, ExecutionError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.pop_value::<DUAL_SLOT>()
        } else {
            self.pop_value::<SINGLE_SLOT>()
        }
    }

    pub(crate) fn get_local<const SLOT: SlotWidth>(
        &self,
        idx: u16,
    ) -> Result<Operand, ExecutionError> {
        let idx = usize::from(idx);
        let lower_slot = self
            .local_variables
            .get(idx)
            .ok_or(ExecutionError::LocalLimitExceed)?;
        let value = match lower_slot {
            Entry::Value(it) => Ok(it.clone()),
            Entry::Top => Err(ExecutionError::ValueMismatch),
            Entry::UninitializedLocal => Err(ExecutionError::LocalUninitialized),
        }?;
        if SLOT == DUAL_SLOT {
            let higher_slot = self
                .local_variables
                .get(idx + 1)
                .ok_or(ExecutionError::LocalLimitExceed)?;
            match higher_slot {
                Entry::Top => Ok(()),
                _ => Err(ExecutionError::ValueMismatch),
            }?;
        }

        Ok(value)
    }

    pub(crate) fn set_local<const SLOT: SlotWidth>(
        &mut self,
        idx: u16,
        value: Operand,
    ) -> Result<(), ExecutionError> {
        let idx = usize::from(idx);
        let lower_slot = self
            .local_variables
            .get_mut(idx)
            .ok_or(ExecutionError::LocalLimitExceed)?;
        *lower_slot = Entry::Value(value);

        if SLOT == DUAL_SLOT {
            let higher_slot = self
                .local_variables
                .get_mut(idx + 1)
                .ok_or(ExecutionError::LocalLimitExceed)?;
            *higher_slot = Entry::Top;
        }

        Ok(())
    }

    pub(crate) fn same_frame(&self) -> Self {
        self.clone()
    }

    pub(crate) fn same_locals_1_stack_item_frame(&self, stack_value: Entry) -> Self {
        let mut operand_stack = Vec::with_capacity(self.max_stack.into());

        operand_stack.push(stack_value);
        Self {
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack,
            possible_ret_addresses: self.possible_ret_addresses.clone(),
        }
    }

    pub(crate) fn merge(&self, other: Self) -> Result<Self, ExecutionError> {
        if self.local_variables.len() != other.local_variables.len() {
            Err(ExecutionError::LocalLimitMismatch)?;
        }
        if self.operand_stack.len() != other.operand_stack.len() {
            Err(ExecutionError::StackSizeMismatch)?;
        }
        let local_variables = self
            .local_variables
            .clone()
            .into_iter()
            .zip(other.local_variables)
            .map(|(lhs, rhs)| lhs.join(rhs))
            .collect();
        let operand_stack = self
            .operand_stack
            .clone()
            .into_iter()
            .zip(other.operand_stack)
            .map(|(lhs, rhs)| lhs.join(rhs))
            .collect();
        let mut possible_ret_addresses = other.possible_ret_addresses;
        possible_ret_addresses.extend(self.possible_ret_addresses.clone());
        Ok(Self {
            max_stack: self.max_stack,
            local_variables,
            operand_stack,
            possible_ret_addresses,
        })
    }
}

fn create_local_variable_entries(
    is_static: bool,
    desc: &MethodDescriptor,
    max_locals: u16,
) -> Result<Box<[Entry]>, ExecutionError> {
    use PrimitiveType::{Double, Long};
    let locals_for_args = desc
        .parameters_types
        .iter()
        .map(|it| match it {
            FieldType::Base(Long | Double) => 2,
            _ => 1,
        })
        .sum::<usize>()
        + usize::from(!is_static);
    if usize::from(max_locals) < locals_for_args {
        return Err(ExecutionError::LocalLimitExceed);
    }
    let this_arg = if is_static {
        None
    } else {
        Some(Entry::Value(Operand::Just(Identifier::This)))
    };
    let args = desc
        .parameters_types
        .iter()
        .enumerate()
        .flat_map(|(arg_idx, local_type)| {
            let arg_idx = u16::try_from(arg_idx).expect("The number of args should be within u16");
            let arg_ref = Operand::Just(Identifier::Arg(arg_idx));
            let maybe_top = if let FieldType::Base(Long | Double) = local_type {
                Some(Entry::Top)
            } else {
                None
            };
            once(Entry::Value(arg_ref)).chain(maybe_top)
        });
    let local_variables = this_arg
        .into_iter()
        .chain(args)
        .pad_using(max_locals.into(), |_| Entry::UninitializedLocal)
        .collect();
    Ok(local_variables)
}
