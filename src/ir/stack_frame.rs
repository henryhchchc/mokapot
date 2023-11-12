use std::{collections::HashSet, fmt::Display, iter::once};

use crate::{
    analysis::fixed_point::FixedPointFact,
    elements::{instruction::ProgramCounter, Method, MethodAccessFlags},
    utils::try_merge,
};

use super::{Identifier, MokaIRGenerationError, ValueRef};

#[derive(PartialEq, Debug)]
pub(super) struct StackFrame {
    max_locals: u16,
    max_stack: u16,
    local_variables: Vec<Option<FrameValue>>,
    operand_stack: Vec<FrameValue>,
    pub reachable_subroutines: HashSet<ProgramCounter>,
}

impl StackFrame {
    pub(super) fn new(method: &Method) -> Self {
        let body = method.body.as_ref().expect("TODO");
        let mut locals = vec![None; body.max_locals.into()];
        let mut local_idx = 0;
        if !method.access_flags.contains(MethodAccessFlags::STATIC) {
            let this_ref = ValueRef::Def(Identifier::This);
            locals[local_idx].replace(FrameValue::ValueRef(this_ref));
            local_idx += 1;
        }
        for i in 0..method.descriptor.parameters_types.len() {
            let arg_ref = ValueRef::Def(Identifier::Arg(i as u16));
            locals[local_idx].replace(FrameValue::ValueRef(arg_ref));
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
            Err(MokaIRGenerationError::StackOverflow)
        } else {
            Ok(self.operand_stack.push(value))
        }
    }

    pub(super) fn pop_raw(&mut self) -> Result<FrameValue, MokaIRGenerationError> {
        self.operand_stack
            .pop()
            .ok_or(MokaIRGenerationError::StackUnderflow)
    }

    pub(super) fn pop_value(&mut self) -> Result<ValueRef, MokaIRGenerationError> {
        match self.pop_raw()? {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Top => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn pop_dual_slot_value(&mut self) -> Result<ValueRef, MokaIRGenerationError> {
        match (self.pop_raw()?, self.pop_raw()?) {
            (FrameValue::ValueRef(it), FrameValue::Top) => Ok(it),
            _ => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn push_value(&mut self, value: ValueRef) -> Result<(), MokaIRGenerationError> {
        self.push_raw(FrameValue::ValueRef(value))
    }

    pub(super) fn push_dual_slot_value(
        &mut self,
        value: ValueRef,
    ) -> Result<(), MokaIRGenerationError> {
        self.push_raw(FrameValue::ValueRef(value))?;
        self.push_raw(FrameValue::Top)
    }

    pub(super) fn get_local(
        &self,
        idx: impl Into<usize>,
    ) -> Result<ValueRef, MokaIRGenerationError> {
        let frame_value = self.local_variables[idx.into()]
            .clone()
            .ok_or(MokaIRGenerationError::LocalUnset)?;
        match frame_value {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Top => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn get_dual_slot_local(
        &self,
        idx: impl Into<usize>,
    ) -> Result<ValueRef, MokaIRGenerationError> {
        let idx = idx.into();
        if idx + 1 >= self.max_locals as usize {
            return Err(MokaIRGenerationError::LocalLimitExceed);
        }
        match (
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].clone(),
            self.local_variables[idx + 1].clone(),
        ) {
            (Some(FrameValue::ValueRef(it)), Some(FrameValue::Top)) => Ok(it),
            _ => Err(MokaIRGenerationError::ValueMismatch),
        }
    }

    pub(super) fn set_local(
        &mut self,
        idx: impl Into<usize>,
        value: ValueRef,
    ) -> Result<(), MokaIRGenerationError> {
        let idx: usize = idx.into();
        if idx < self.max_locals as usize {
            self.local_variables[idx].replace(FrameValue::ValueRef(value));
            Ok(())
        } else {
            Err(MokaIRGenerationError::LocalLimitExceed)
        }
    }

    pub(super) fn set_dual_slot_local(
        &mut self,
        idx: impl Into<usize>,
        value: ValueRef,
    ) -> Result<(), MokaIRGenerationError> {
        let idx: usize = idx.into();
        if idx + 1 < self.max_locals as usize {
            // If panic here then `local_variables` are not allocated correctly
            self.local_variables[idx].replace(FrameValue::ValueRef(value));
            self.local_variables[idx + 1].replace(FrameValue::Top);
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

    fn merge(&self, other: Self) -> Result<Self, Self::MergeError> {
        if self.max_locals != other.max_locals {
            return Err(MokaIRGenerationError::LocalLimitMismatch);
        }
        if self.local_variables.len() != other.local_variables.len() {
            panic!("BUG: `local_variables` are not allocated correctly")
        }
        if self.operand_stack.len() != other.operand_stack.len() {
            return Err(MokaIRGenerationError::StackSizeMismatch);
        }
        let reachable_subroutines = self
            .reachable_subroutines
            .clone()
            .into_iter()
            .chain(other.reachable_subroutines)
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
            reachable_subroutines,
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
