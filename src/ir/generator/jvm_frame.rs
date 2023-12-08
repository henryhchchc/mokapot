use std::{collections::BTreeSet, fmt::Display};

use crate::{
    ir::{Argument, Identifier},
    jvm::{code::ProgramCounter, method::MethodDescriptor},
    types::field_type::{FieldType, PrimitiveType},
};

#[derive(PartialEq, Debug)]
pub(super) struct JvmStackFrame {
    max_locals: u16,
    max_stack: u16,
    local_variables: Vec<Option<Entry>>,
    operand_stack: Vec<Entry>,
    pub possible_ret_addresses: BTreeSet<ProgramCounter>,
}

#[derive(Debug, thiserror::Error)]
pub enum JvmFrameError {
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    #[error("The stack size exceeds the max stack size")]
    StackOverflow,
    #[error("The local variable index exceeds the max local variable size")]
    LocalLimitExceed,
    #[error("The local variable is not initialized")]
    LocalUnset,
    #[error("The stack size mismatch")]
    StackSizeMismatch,
    #[error("The local limit mismatch")]
    LocalLimitMismatch,
    #[error("Expected a ValueRef but got a Padding or vice versa")]
    ValueMismatch,
}

impl JvmStackFrame {
    pub(super) fn new(
        is_static: bool,
        desc: MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
    ) -> Self {
        let mut local_variables = vec![None; max_locals.into()];
        let mut local_idx = 0;
        if !is_static {
            let this_ref = Argument::Id(Identifier::This);
            local_variables[local_idx].replace(Entry::Value(this_ref));
            local_idx += 1;
        }
        for (arg_idx, local_type) in desc.parameters_types.into_iter().enumerate() {
            let arg_ref = Argument::Id(Identifier::Arg(
                u16::try_from(arg_idx).expect("The number of args should be within u16"),
            ));
            local_variables[local_idx].replace(Entry::Value(arg_ref));
            local_idx += 1;
            if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = local_type {
                local_variables[local_idx].replace(Entry::Top);
                local_idx += 1;
            }
        }
        JvmStackFrame {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack as usize),
            possible_ret_addresses: BTreeSet::new(),
        }
    }

    #[inline]
    fn push_raw(&mut self, value: Entry) -> Result<(), JvmFrameError> {
        if u16::try_from(self.operand_stack.len()).expect("The stack size should be within u16")
            >= self.max_stack
        {
            Err(JvmFrameError::StackOverflow)
        } else {
            self.operand_stack.push(value);
            Ok(())
        }
    }

    #[inline]
    fn pop_raw(&mut self) -> Result<Entry, JvmFrameError> {
        self.operand_stack
            .pop()
            .ok_or(JvmFrameError::StackUnderflow)
    }

    pub(super) fn pop_value(&mut self) -> Result<Argument, JvmFrameError> {
        match self.pop_raw()? {
            Entry::Value(it) => Ok(it),
            Entry::Top => Err(JvmFrameError::ValueMismatch),
        }
    }

    pub(super) fn pop_dual_slot_value(&mut self) -> Result<Argument, JvmFrameError> {
        match (self.pop_raw()?, self.pop_raw()?) {
            (Entry::Value(it), Entry::Top) => Ok(it),
            _ => Err(JvmFrameError::ValueMismatch),
        }
    }

    pub(super) fn push_value(&mut self, value: Argument) -> Result<(), JvmFrameError> {
        self.push_raw(Entry::Value(value))
    }

    pub(super) fn push_dual_slot_value(&mut self, value: Argument) -> Result<(), JvmFrameError> {
        self.push_raw(Entry::Top)?;
        self.push_raw(Entry::Value(value))
    }

    pub(super) fn get_local(&self, idx: impl Into<usize>) -> Result<Argument, JvmFrameError> {
        let idx = idx.into();
        let frame_value = self.local_variables[idx]
            .clone()
            .ok_or(JvmFrameError::LocalUnset)?;
        match frame_value {
            Entry::Value(it) => Ok(it),
            Entry::Top => Err(JvmFrameError::ValueMismatch),
        }
    }

    pub(super) fn pop_args(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<Argument>, JvmFrameError> {
        use FieldType::Base;
        use PrimitiveType::{Double, Long};
        let mut args = Vec::with_capacity(descriptor.parameters_types.len());
        for param_type in descriptor.parameters_types.iter().rev() {
            let arg = if let Base(Long | Double) = param_type {
                self.pop_dual_slot_value()?
            } else {
                self.pop_value()?
            };
            args.push(arg);
        }
        args.reverse();
        Ok(args)
    }

    pub(super) fn typed_push(
        &mut self,
        value_type: &FieldType,
        value: Argument,
    ) -> Result<(), JvmFrameError> {
        match value_type {
            FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) => {
                self.push_dual_slot_value(value)
            }
            _ => self.push_value(value),
        }
    }

    pub(super) fn get_dual_slot_local(
        &self,
        idx: impl Into<usize>,
    ) -> Result<Argument, JvmFrameError> {
        let idx = idx.into();
        if idx + 1 >= self.max_locals as usize {
            Err(JvmFrameError::LocalLimitExceed)?;
        }
        match (
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].as_ref(),
            self.local_variables[idx + 1].as_ref(),
        ) {
            (Some(Entry::Value(it)), Some(Entry::Top)) => Ok(it.clone()),
            _ => Err(JvmFrameError::ValueMismatch),
        }
    }

    pub(super) fn set_local(
        &mut self,
        idx: impl Into<usize>,
        value: Argument,
    ) -> Result<(), JvmFrameError> {
        let idx: usize = idx.into();
        if idx < self.max_locals as usize {
            self.local_variables[idx].replace(Entry::Value(value));
            if idx < self.max_locals as usize - 1
                && matches!(self.local_variables[idx + 1], Some(Entry::Top))
            {
                self.local_variables[idx + 1].take();
            }
            Ok(())
        } else {
            Err(JvmFrameError::LocalLimitExceed)
        }
    }

    pub(super) fn set_dual_slot_local(
        &mut self,
        idx: impl Into<usize>,
        value: Argument,
    ) -> Result<(), JvmFrameError> {
        let idx: usize = idx.into();
        if idx + 1 < self.max_locals as usize {
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].replace(Entry::Value(value));
            self.local_variables[idx + 1].replace(Entry::Top);
            Ok(())
        } else {
            Err(JvmFrameError::LocalLimitExceed)
        }
    }

    pub(super) fn same_frame(&self) -> Self {
        Self {
            max_locals: self.max_locals,
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack: self.operand_stack.clone(),
            possible_ret_addresses: self.possible_ret_addresses.clone(),
        }
    }

    pub(super) fn same_locals_1_stack_item_frame(&self, stack_value: Entry) -> Self {
        let mut operand_stack = Vec::with_capacity(self.max_stack as usize);
        operand_stack.push(stack_value);
        Self {
            max_locals: self.max_locals,
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack,
            possible_ret_addresses: self.possible_ret_addresses.clone(),
        }
    }
}

/// Implementations of JVM stack frame instructions.
impl JvmStackFrame {
    pub(super) fn pop(&mut self) -> Result<(), JvmFrameError> {
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    pub(super) fn pop2(&mut self) -> Result<(), JvmFrameError> {
        let _top_element = self.pop_raw()?;
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    pub(super) fn dup(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup_x1(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup_x2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup2_x1(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup2_x2(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        let fourth_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(fourth_element)?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn swap(&mut self) -> Result<(), JvmFrameError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element)?;
        self.push_raw(second_element)?;
        Ok(())
    }
}

impl JvmStackFrame {
    pub(super) fn merge(&self, other: Self) -> Result<Self, JvmFrameError> {
        if self.max_locals != other.max_locals {
            Err(JvmFrameError::LocalLimitMismatch)?;
        }
        assert!(
            self.local_variables.len() == other.local_variables.len(),
            "BUG: `local_variables` are not allocated correctly"
        );
        if self.operand_stack.len() != other.operand_stack.len() {
            Err(JvmFrameError::StackSizeMismatch)?;
        }
        let reachable_subroutines = self
            .possible_ret_addresses
            .clone()
            .into_iter()
            .chain(other.possible_ret_addresses)
            .collect();
        let local_variables = self
            .local_variables
            .clone()
            .into_iter()
            .zip(other.local_variables)
            .map(|(self_loc, other_loc)| match (self_loc, other_loc) {
                (Some(self_loc), Some(other_loc)) => Some(Entry::merge(self_loc, other_loc)),
                (lhs, rhs) => lhs.or(rhs),
            })
            .collect();
        let operand_stack = self
            .operand_stack
            .clone()
            .into_iter()
            .zip(other.operand_stack)
            .map(|(lhs, rhs)| Entry::merge(lhs, rhs))
            .collect();
        Ok(Self {
            max_locals: self.max_locals,
            max_stack: self.max_stack,
            local_variables,
            operand_stack,
            possible_ret_addresses: reachable_subroutines,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum Entry {
    Value(Argument),
    Top,
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(id) => id.fmt(f),
            Self::Top => write!(f, "Top"),
        }
    }
}

impl Entry {
    pub fn merge(x: Self, y: Self) -> Self {
        match (x, y) {
            (Entry::Value(lhs), Entry::Value(rhs)) => Entry::Value(lhs | rhs),
            // NOTE: `lhs` and `rhs` are different variants, that means the local variable slot is reused
            //       In this case, we do not merge it since it will be overridden afterwrds.
            (lhs, _) => lhs,
        }
    }
}

#[cfg(test)]
mod test {
    use std::collections::BTreeSet;

    use crate::ir::{Argument, Identifier, Value};

    use super::Entry;

    #[test]
    fn merge_value_ref() {
        let lhs = Entry::Value(Argument::Id(Identifier::Value(Value::new(0))));
        let rhs = Entry::Value(Argument::Id(Identifier::Value(Value::new(1))));

        let result = Entry::merge(lhs, rhs);
        assert_eq!(
            result,
            Entry::Value(Argument::Phi(BTreeSet::from([
                Identifier::Value(Value::new(0)),
                Identifier::Value(Value::new(1))
            ])))
        );
    }

    #[test]
    fn merge_same_value_ref() {
        let lhs = Entry::Value(Argument::Id(Identifier::Value(Value::new(0))));
        let rhs = Entry::Value(Argument::Id(Identifier::Value(Value::new(0))));

        let result = Entry::merge(lhs, rhs);
        assert_eq!(
            result,
            Entry::Value(Argument::Id(Identifier::Value(Value::new(0))))
        );
    }
}
