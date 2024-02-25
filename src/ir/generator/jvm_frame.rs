use std::{collections::BTreeSet, fmt::Display, iter::once};

use itertools::Itertools;

use crate::{
    ir::{Argument, Identifier},
    jvm::code::ProgramCounter,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};

#[derive(PartialEq, Debug, Clone)]
pub(super) struct JvmStackFrame {
    max_locals: u16,
    max_stack: u16,
    local_variables: Vec<Entry>,
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
    LocalUninitialized,
    #[error("The stack size mismatch")]
    StackSizeMismatch,
    #[error("The local limit mismatch")]
    LocalLimitMismatch,
    #[error("Value type in the stack or local variable table mismatch")]
    ValueMismatch,
}

impl JvmStackFrame {
    pub(super) fn new(
        is_static: bool,
        desc: &MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
    ) -> Result<Self, JvmFrameError> {
        if usize::from(max_locals) < usize::from(is_static) + desc.parameters_types.len() {
            return Err(JvmFrameError::LocalLimitExceed);
        }
        let this_arg = if is_static {
            None
        } else {
            Some(Entry::Value(Argument::Id(Identifier::This)))
        };
        let args = desc
            .parameters_types
            .iter()
            .enumerate()
            .flat_map(|(arg_idx, local_type)| {
                use PrimitiveType::{Double, Long};
                let arg_idx =
                    u16::try_from(arg_idx).expect("The number of args should be within u16");
                let arg_ref = Argument::Id(Identifier::Arg(arg_idx));
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
        Ok(Self {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack.into()),
            possible_ret_addresses: BTreeSet::new(),
        })
    }

    #[inline]
    fn push_raw(&mut self, value: Entry) -> Result<(), JvmFrameError> {
        let stack_size =
            u16::try_from(self.operand_stack.len()).expect("The stack size should be within u16");
        if stack_size >= self.max_stack {
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
            // `UninitializedLocal` is never pushed to the stack
            Entry::UninitializedLocal => unreachable!(),
        }
    }

    pub(super) fn pop_dual_slot_value(&mut self) -> Result<Argument, JvmFrameError> {
        match (self.pop_raw()?, self.pop_raw()?) {
            (Entry::Value(it), Entry::Top) => Ok(it),
            // `UninitializedLocal` is never pushed to the stack
            (Entry::UninitializedLocal, _) | (_, Entry::UninitializedLocal) => unreachable!(),
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

    pub(super) fn get_local(&self, idx: impl Into<u16>) -> Result<Argument, JvmFrameError> {
        let idx = idx.into();
        let frame_value = &self.local_variables[usize::from(idx)];
        match frame_value {
            Entry::Value(it) => Ok(it.clone()),
            Entry::Top => Err(JvmFrameError::ValueMismatch),
            Entry::UninitializedLocal => Err(JvmFrameError::LocalUninitialized),
        }
    }

    pub(super) fn get_dual_slot_local(
        &self,
        idx: impl Into<u16>,
    ) -> Result<Argument, JvmFrameError> {
        let idx = idx.into();
        if idx + 1 >= self.max_locals {
            Err(JvmFrameError::LocalLimitExceed)?;
        }
        match (
            // Panic only when `local_variables` were not allocated correctly
            &self.local_variables[usize::from(idx)],
            &self.local_variables[usize::from(idx + 1)],
        ) {
            (Entry::Value(it), Entry::Top) => Ok(it.clone()),
            (Entry::UninitializedLocal, _) | (_, Entry::UninitializedLocal) => {
                Err(JvmFrameError::LocalUninitialized)
            }
            _ => Err(JvmFrameError::ValueMismatch),
        }
    }

    pub(super) fn set_local(
        &mut self,
        idx: impl Into<u16>,
        value: Argument,
    ) -> Result<(), JvmFrameError> {
        let idx = idx.into();
        if idx >= self.max_locals {
            Err(JvmFrameError::LocalLimitExceed)?;
        }
        self.local_variables[usize::from(idx)] = Entry::Value(value);
        if idx + 1 < self.max_locals
            && matches!(self.local_variables[usize::from(idx + 1)], Entry::Top)
        {
            self.local_variables[usize::from(idx + 1)].erase();
        }
        Ok(())
    }

    pub(super) fn set_dual_slot_local(
        &mut self,
        idx: impl Into<u16>,
        value: Argument,
    ) -> Result<(), JvmFrameError> {
        let idx = idx.into();
        if idx + 1 < self.max_locals {
            // Panic only when `local_variables` were not allocated correctly
            self.local_variables[usize::from(idx)] = Entry::Value(value);
            self.local_variables[usize::from(idx + 1)] = Entry::Top;
            Ok(())
        } else {
            Err(JvmFrameError::LocalLimitExceed)
        }
    }

    pub(super) fn same_frame(&self) -> Self {
        self.clone()
    }

    pub(super) fn same_locals_1_stack_item_frame(&self, stack_value: Entry) -> Self {
        let mut operand_stack = Vec::with_capacity(self.max_stack.into());

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
        debug_assert!(
            self.local_variables.len() == other.local_variables.len(),
            "The size of the local variables does not match"
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
            .map(|(lhs, rhs)| Entry::merge(lhs, rhs))
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
    UninitializedLocal,
}

impl Display for Entry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Value(id) => id.fmt(f),
            Self::Top => write!(f, "Top"),
            Self::UninitializedLocal => write!(f, "<uninitialized_local>"),
        }
    }
}

impl Entry {
    pub fn merge(lhs: Self, rhs: Self) -> Self {
        #[allow(clippy::enum_glob_use)]
        use Entry::*;
        match (lhs, rhs) {
            (Value(lhs), Value(rhs)) => Value(lhs | rhs),
            (Top, Top) => Top,
            (UninitializedLocal, it) | (it, UninitializedLocal) => it,
            // NOTE: When `lhs` and `rhs` are different variants, it indicates that the local
            //       variable slot is reused. In this case, we do not merge it since it will be
            //       overridden afterwrds.
            (lhs, _) => lhs,
        }
    }

    pub fn erase(&mut self) {
        *self = Self::UninitializedLocal;
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
