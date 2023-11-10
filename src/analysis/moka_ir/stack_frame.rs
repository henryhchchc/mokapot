use std::{cmp::max, collections::HashSet, fmt::Display, iter::once};

use crate::{
    analysis::jvm_fixed_point::FixedPointFact,
    elements::{instruction::ProgramCounter, Method, MethodAccessFlags},
    utils::try_merge,
};

use super::{Identifier, MokaIRGenerationError, ValueRef};

#[derive(PartialEq, Debug)]
pub(super) struct StackFrame {
    pub max_locals: u16,
    pub max_stack: u16,
    local_variables: Vec<Option<FrameValue>>,
    operand_stack: Vec<FrameValue>,
    pub reachable_subroutines: HashSet<ProgramCounter>,
}

impl StackFrame {
    pub(super) fn new(method: &Method) -> Self {
        let body = method.body.as_ref().expect("TODO");
        let mut locals = Vec::with_capacity(body.max_locals as usize);
        for _ in 0..body.max_locals {
            locals.push(None);
        }
        let mut local_idx = 0;
        if !method.access_flags.contains(MethodAccessFlags::STATIC) {
            locals[local_idx].replace(Identifier::This.into());
            local_idx += 1;
        }
        for i in 0..method.descriptor.parameters_types.len() {
            locals[local_idx].replace(Identifier::Arg(i as u16).into());
            local_idx += 1;
        }
        StackFrame {
            max_locals: body.max_locals,
            max_stack: body.max_stack,
            local_variables: locals,
            operand_stack: Vec::with_capacity(body.max_stack as usize),
            reachable_subroutines: HashSet::new(),
        }
    }

    pub(super) fn push_raw(&mut self, value: FrameValue) -> Result<(), MokaIRGenerationError> {
        if self.operand_stack.len() as u16 >= self.max_stack {
            return Err(MokaIRGenerationError::StackOverflow);
        }
        Ok(self.operand_stack.push(value))
    }

    pub(super) fn pop_raw(&mut self) -> Result<FrameValue, MokaIRGenerationError> {
        self.operand_stack
            .pop()
            .ok_or(MokaIRGenerationError::StackUnderflow)
    }

    pub(super) fn pop_value(&mut self) -> Result<ValueRef, MokaIRGenerationError> {
        match self.pop_raw()? {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Padding => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn pop_padding(&mut self) -> Result<(), MokaIRGenerationError> {
        match self.pop_raw()? {
            FrameValue::ValueRef(_) => Err(MokaIRGenerationError::ValueMismatch),
            FrameValue::Padding => Ok(()),
        }
    }

    pub(super) fn push_value(&mut self, value: ValueRef) -> Result<(), MokaIRGenerationError> {
        self.push_raw(FrameValue::ValueRef(value))
    }

    pub(super) fn push_padding(&mut self) -> Result<(), MokaIRGenerationError> {
        self.push_raw(FrameValue::Padding)
    }

    pub(super) fn get_local(
        &self,
        idx: impl Into<usize>,
    ) -> Result<ValueRef, MokaIRGenerationError> {
        let frame_value = self
            .local_variables
            .get(idx.into())
            .expect("BUG: `local_variables` is not allocated correctly")
            .clone()
            .ok_or(MokaIRGenerationError::LocalUnset)?;
        match frame_value {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Padding => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn set_local(
        &mut self,
        idx: impl Into<usize>,
        value: ValueRef,
    ) -> Result<(), MokaIRGenerationError> {
        let idx = idx.into();
        if idx <= self.max_locals as usize {
            self.local_variables
                .get_mut(idx)
                .expect("BUG: `local_variables` is not allocated correctly")
                .replace(FrameValue::ValueRef(value));
            Ok(())
        } else {
            Err(MokaIRGenerationError::LocalLimitExceed)
        }
    }

    pub(super) fn set_local_padding(
        &mut self,
        idx: impl Into<usize>,
    ) -> Result<(), MokaIRGenerationError> {
        let idx = idx.into();
        if idx <= self.max_locals as usize {
            self.local_variables
                .get_mut(idx)
                .expect("BUG: `local_variables` is not allocated correctly")
                .replace(FrameValue::Padding);
            Ok(())
        } else {
            Err(MokaIRGenerationError::LocalLimitExceed)
        }
    }

    pub(super) fn same_frame(&self) -> Self {
        Self {
            max_locals: self.max_locals,
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack: self.operand_stack.clone(),
            reachable_subroutines: self.reachable_subroutines.clone(),
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
            reachable_subroutines: self.reachable_subroutines.clone(),
        }
    }
}

impl FixedPointFact for StackFrame {
    type MergeError = MokaIRGenerationError;

    fn merge(&self, other: &Self) -> Result<Self, Self::MergeError> {
        let max_locals = max(self.max_locals, other.max_locals);
        let max_stack = max(self.max_stack, other.max_stack);
        let reachable_subroutines = self
            .reachable_subroutines
            .clone()
            .into_iter()
            .chain(other.reachable_subroutines.clone())
            .collect();
        let mut local_variables = Vec::with_capacity(max_locals as usize);
        for i in 0..max_locals as usize {
            local_variables.insert(i, None);
            let self_loc = self.local_variables.get(i).cloned();
            let other_loc = other.local_variables.get(i).cloned();
            local_variables[i] = match (self_loc, other_loc) {
                (Some(x), Some(y)) => try_merge(x, y, FrameValue::merge)?,
                (x, y) => x
                    .or(y)
                    .expect("The local variable vec is not allocated correctly"),
            }
        }
        let mut operand_stack = Vec::with_capacity(max_stack as usize);
        for i in 0..max(self.operand_stack.len(), other.operand_stack.len()) as usize {
            let self_loc = self.operand_stack.get(i).cloned();
            let other_loc = other.operand_stack.get(i).cloned();
            let stack_value = match (self_loc, other_loc) {
                (Some(x), Some(y)) => FrameValue::merge(x, y)?,
                (x, y) => x.or(y).expect("BUG: The stack is not allocated correctly"),
            };
            operand_stack.push(stack_value);
        }

        Ok(Self {
            max_locals,
            max_stack,
            local_variables,
            operand_stack,
            reachable_subroutines,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrameValue {
    ValueRef(ValueRef),
    Padding,
}

impl From<Identifier> for FrameValue {
    fn from(value: Identifier) -> Self {
        Self::ValueRef(ValueRef::Def(value))
    }
}

impl Display for FrameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FrameValue::*;
        match self {
            ValueRef(id) => write!(f, "{}", id),
            Padding => write!(f, "Padding"),
        }
    }
}

impl FrameValue {
    pub fn merge(x: Self, y: Self) -> Result<Self, MokaIRGenerationError> {
        use ValueRef::*;
        match (x, y) {
            (lhs, rhs) if lhs == rhs => Ok(lhs),
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
            _ => Err(MokaIRGenerationError::ValueMismatch),
        }
    }
}
