use std::{collections::HashSet, fmt::Display, iter::once};

use itertools::Itertools;

use crate::{
    analysis::fixed_point::FixedPointFact,
    elements::{instruction::ProgramCounter, MethodDescriptor},
    ir::{Identifier, ValueRef},
    types::{FieldType, PrimitiveType},
    utils::try_merge,
};

#[derive(PartialEq, Debug)]
pub(super) struct StackFrame {
    max_locals: u16,
    max_stack: u16,
    local_variables: Vec<Option<FrameValue>>,
    operand_stack: Vec<FrameValue>,
    pub possible_ret_addresses: HashSet<ProgramCounter>,
}

#[derive(Debug, thiserror::Error)]
pub enum StackFrameError {
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

impl StackFrame {
    pub(super) fn new(
        is_static: bool,
        desc: MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
    ) -> Self {
        let mut local_variables = vec![None; max_locals.into()];
        let mut local_idx = 0;
        if !is_static {
            let this_ref = ValueRef::Def(Identifier::This);
            local_variables[local_idx].replace(FrameValue::ValueRef(this_ref));
            local_idx += 1;
        }
        for (idx, local_type) in desc.parameters_types.iter().with_position() {
            let arg_ref = ValueRef::Def(Identifier::Arg(idx as u16));
            local_variables[local_idx].replace(FrameValue::ValueRef(arg_ref));
            local_idx += 1;
            if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = local_type {
                local_variables[local_idx].replace(FrameValue::Top);
                local_idx += 1;
            }
        }
        StackFrame {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack as usize),
            possible_ret_addresses: HashSet::new(),
        }
    }

    pub(super) fn push_raw(&mut self, value: FrameValue) -> Result<(), StackFrameError> {
        if self.operand_stack.len() as u16 >= self.max_stack {
            Err(StackFrameError::StackOverflow)
        } else {
            Ok(self.operand_stack.push(value))
        }
    }

    pub(super) fn pop_raw(&mut self) -> Result<FrameValue, StackFrameError> {
        self.operand_stack
            .pop()
            .ok_or(StackFrameError::StackUnderflow)
    }

    pub(super) fn pop_value(&mut self) -> Result<ValueRef, StackFrameError> {
        match self.pop_raw()? {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Top => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(super) fn pop_dual_slot_value(&mut self) -> Result<ValueRef, StackFrameError> {
        match (self.pop_raw()?, self.pop_raw()?) {
            (FrameValue::ValueRef(it), FrameValue::Top) => Ok(it),
            _ => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(super) fn push_value(&mut self, value: ValueRef) -> Result<(), StackFrameError> {
        self.push_raw(FrameValue::ValueRef(value))
    }

    pub(super) fn push_dual_slot_value(&mut self, value: ValueRef) -> Result<(), StackFrameError> {
        self.push_raw(FrameValue::Top)?;
        self.push_raw(FrameValue::ValueRef(value))
    }

    pub(super) fn get_local(&self, idx: impl Into<usize>) -> Result<ValueRef, StackFrameError> {
        let idx = idx.into();
        let frame_value = self.local_variables[idx]
            .clone()
            .ok_or(StackFrameError::LocalUnset)?;
        match frame_value {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Top => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(super) fn typed_pop(
        &mut self,
        value_type: &FieldType,
    ) -> Result<ValueRef, StackFrameError> {
        match value_type {
            FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) => {
                self.pop_dual_slot_value()
            }
            _ => self.pop_value(),
        }
    }

    pub(super) fn typed_push(
        &mut self,
        value_type: &FieldType,
        value: ValueRef,
    ) -> Result<(), StackFrameError> {
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
    ) -> Result<ValueRef, StackFrameError> {
        let idx = idx.into();
        if idx + 1 >= self.max_locals as usize {
            return Err(StackFrameError::LocalLimitExceed);
        }
        match (
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].as_ref(),
            self.local_variables[idx + 1].as_ref(),
        ) {
            (Some(FrameValue::ValueRef(it)), Some(FrameValue::Top)) => Ok(it.clone()),
            _ => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(super) fn set_local(
        &mut self,
        idx: impl Into<usize>,
        value: ValueRef,
    ) -> Result<(), StackFrameError> {
        let idx: usize = idx.into();
        if idx < self.max_locals as usize {
            self.local_variables[idx].replace(FrameValue::ValueRef(value));
            if idx < self.max_locals as usize - 1
                && matches!(self.local_variables[idx + 1], Some(FrameValue::Top))
            {
                self.local_variables[idx + 1].take();
            }
            Ok(())
        } else {
            Err(StackFrameError::LocalLimitExceed)
        }
    }

    pub(super) fn set_dual_slot_local(
        &mut self,
        idx: impl Into<usize>,
        value: ValueRef,
    ) -> Result<(), StackFrameError> {
        let idx: usize = idx.into();
        if idx + 1 < self.max_locals as usize {
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].replace(FrameValue::ValueRef(value));
            self.local_variables[idx + 1].replace(FrameValue::Top);
            Ok(())
        } else {
            Err(StackFrameError::LocalLimitExceed)
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

    pub(super) fn same_locals_1_stack_item_frame(&self, stack_value: FrameValue) -> Self {
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

impl FixedPointFact for StackFrame {
    type MergeError = StackFrameError;

    fn merge(&self, other: Self) -> Result<Self, Self::MergeError> {
        if self.max_locals != other.max_locals {
            return Err(StackFrameError::LocalLimitMismatch);
        }
        if self.local_variables.len() != other.local_variables.len() {
            panic!("BUG: `local_variables` are not allocated correctly")
        }
        if self.operand_stack.len() != other.operand_stack.len() {
            return Err(StackFrameError::StackSizeMismatch);
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
            .map(|(self_loc, other_loc)| try_merge(self_loc, other_loc, FrameValue::merge))
            .collect::<Result<_, _>>()?;
        Ok(Self {
            max_locals: self.max_locals,
            max_stack: self.max_stack,
            local_variables,
            operand_stack: self.operand_stack.clone(),
            possible_ret_addresses: reachable_subroutines,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub(super) enum FrameValue {
    ValueRef(ValueRef),
    Top,
}

impl Display for FrameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FrameValue::*;
        match self {
            ValueRef(id) => id.fmt(f),
            Top => write!(f, "Top"),
        }
    }
}

impl FrameValue {
    pub fn merge(x: Self, y: Self) -> Result<Self, StackFrameError> {
        use ValueRef::*;
        match (x, y) {
            (FrameValue::ValueRef(lhs), FrameValue::ValueRef(rhs)) => {
                let result = match (lhs, rhs) {
                    (Def(id_x), Def(id_y)) => Phi(HashSet::from([id_x, id_y])),
                    (Def(id), Phi(ids)) | (Phi(ids), Def(id)) => {
                        Phi(ids.into_iter().chain(once(id)).collect())
                    }
                    (Phi(ids_x), Phi(ids_y)) => Phi(ids_x.into_iter().chain(ids_y).collect()),
                };
                Ok(FrameValue::ValueRef(result))
            }
            // NOTE: `lhs` and `rhs` are different variants, that means the local variable slot is reused
            //       In this case, we do not merge it since it will be overridden afterwrds.
            (lhs, _) => Ok(lhs),
        }
    }
}
