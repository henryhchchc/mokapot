use std::iter::once;

use itertools::Itertools;

use crate::{
    analysis::fixed_point::JoinSemiLattice,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};

use super::super::DiscoveryValue;

pub(crate) type SlotWidth = bool;
pub(crate) const SINGLE_SLOT: SlotWidth = false;
pub(crate) const DUAL_SLOT: SlotWidth = true;

use super::{entry::Entry, error::ExecutionError};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct JvmStackFrame<V = DiscoveryValue> {
    max_stack: u16,
    local_variables: Box<[Entry<V>]>,
    operand_stack: Vec<Entry<V>>,
}

impl<V: Clone + PartialOrd> PartialOrd for JvmStackFrame<V> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        use std::cmp::Ordering::Equal;
        if self.max_stack != other.max_stack {
            return None;
        }
        if self.operand_stack.len() != other.operand_stack.len() {
            return None;
        }
        let stack_order = self.operand_stack.partial_cmp(&other.operand_stack);
        let locals_order = self.local_variables.partial_cmp(&other.local_variables);
        match (stack_order, locals_order) {
            (Some(Equal), ord) | (ord, Some(Equal)) => ord,
            (ord @ Some(s_ord), Some(l_ord)) if s_ord == l_ord => ord,
            _ => None,
        }
    }
}

impl<V: Clone + JoinSemiLattice> JoinSemiLattice for JvmStackFrame<V> {
    /// Joins two stack frames by merging their local variables and operand stacks.
    ///
    /// # Panics
    ///
    /// This function panics if the local variables or operand stacks of the two stack frames have different lengths.
    fn join(self, other: Self) -> Self {
        let local_variables = self
            .local_variables
            .clone()
            .into_iter()
            .zip_eq(other.local_variables)
            .map(|(lhs, rhs)| lhs.join(rhs))
            .collect();
        let operand_stack = self
            .operand_stack
            .clone()
            .into_iter()
            .zip_eq(other.operand_stack)
            .map(|(lhs, rhs)| lhs.join(rhs))
            .collect();
        Self {
            max_stack: self.max_stack,
            local_variables,
            operand_stack,
        }
    }
}

impl JvmStackFrame<DiscoveryValue> {
    pub(crate) fn new(
        is_static: bool,
        desc: &MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
    ) -> Result<Self, ExecutionError> {
        let this_value = (!is_static).then_some(DiscoveryValue::This);
        let parameters = desc
            .parameters_types
            .iter()
            .enumerate()
            .map(|(index, _)| {
                DiscoveryValue::Arg(
                    u16::try_from(index).expect("descriptor parameter count fits u16"),
                )
            })
            .collect::<Vec<_>>();
        Self::with_inputs(desc, max_locals, max_stack, this_value, &parameters)
    }
}

impl<V: Clone> JvmStackFrame<V> {
    pub(crate) fn with_inputs(
        desc: &MethodDescriptor,
        max_locals: u16,
        max_stack: u16,
        this_value: Option<V>,
        parameters: &[V],
    ) -> Result<Self, ExecutionError> {
        if parameters.len() != desc.parameters_types.len() {
            return Err(ExecutionError::ValueMismatch);
        }
        let local_variables =
            create_local_variable_entries(desc, max_locals, this_value, parameters)?;
        Ok(Self {
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack.into()),
        })
    }

    pub(crate) fn pop_raw(&mut self) -> Result<Entry<V>, ExecutionError> {
        self.operand_stack
            .pop()
            .ok_or(ExecutionError::StackUnderflow)
    }

    pub(crate) fn push_raw(&mut self, value: Entry<V>) -> Result<(), ExecutionError> {
        let stack_size =
            u16::try_from(self.operand_stack.len()).expect("The stack size should be within u16");
        if stack_size >= self.max_stack {
            Err(ExecutionError::StackOverflow)
        } else {
            self.operand_stack.push(value);
            Ok(())
        }
    }

    pub(crate) fn pop_value<const SLOT: SlotWidth>(&mut self) -> Result<V, ExecutionError> {
        let value = match self.pop_raw()? {
            Entry::Value(it) => Ok(it),
            Entry::Top => Err(ExecutionError::ValueMismatch),
            Entry::UninitializedLocal | Entry::OutOfScope => {
                unreachable!("It is never pushed to the stack")
            }
        }?;
        if SLOT == DUAL_SLOT {
            match self.pop_raw()? {
                Entry::Top => Ok(()),
                Entry::Value(_) => Err(ExecutionError::ValueMismatch),
                Entry::UninitializedLocal | Entry::OutOfScope => {
                    unreachable!("It is never pushed to the stack")
                }
            }?;
        }
        Ok(value)
    }

    pub(crate) fn push_value<const SLOT: SlotWidth>(
        &mut self,
        value: V,
    ) -> Result<(), ExecutionError> {
        if SLOT == DUAL_SLOT {
            self.push_raw(Entry::Top)?;
        }
        self.push_raw(Entry::Value(value))
    }

    pub(crate) fn pop_args(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<V>, ExecutionError> {
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
        value: V,
    ) -> Result<(), ExecutionError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.push_value::<DUAL_SLOT>(value)
        } else {
            self.push_value::<SINGLE_SLOT>(value)
        }
    }

    pub(crate) fn typed_pop(&mut self, value_type: &FieldType) -> Result<V, ExecutionError> {
        if let FieldType::Base(PrimitiveType::Long | PrimitiveType::Double) = value_type {
            self.pop_value::<DUAL_SLOT>()
        } else {
            self.pop_value::<SINGLE_SLOT>()
        }
    }

    pub(crate) fn get_local<const SLOT: SlotWidth>(&self, idx: u16) -> Result<V, ExecutionError> {
        let idx = usize::from(idx);
        let lower_slot = self
            .local_variables
            .get(idx)
            .ok_or(ExecutionError::LocalLimitExceed)?;
        let value = match lower_slot {
            Entry::Value(it) => Ok(it.clone()),
            Entry::Top => Err(ExecutionError::ValueMismatch),
            Entry::OutOfScope => Err(ExecutionError::LocalOutOfScope),
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
        value: V,
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

    pub(crate) fn same_locals_1_stack_item_frame(&self, stack_value: Entry<V>) -> Self {
        let mut operand_stack = Vec::with_capacity(self.max_stack.into());

        operand_stack.push(stack_value);
        Self {
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack,
        }
    }

    pub(crate) fn same_locals_empty_stack_frame(&self) -> Self {
        Self {
            max_stack: self.max_stack,
            local_variables: self.local_variables.clone(),
            operand_stack: Vec::with_capacity(self.max_stack.into()),
        }
    }

    pub(crate) fn without_values(&self) -> Self {
        Self {
            max_stack: self.max_stack,
            local_variables: self
                .local_variables
                .iter()
                .map(|entry| match entry {
                    Entry::Value(_) | Entry::UninitializedLocal => Entry::UninitializedLocal,
                    Entry::Top => Entry::Top,
                    Entry::OutOfScope => Entry::OutOfScope,
                })
                .collect(),
            operand_stack: Vec::with_capacity(self.max_stack.into()),
        }
    }

    pub(crate) fn try_map_values<U: Clone, E>(
        &self,
        mut map: impl FnMut(&V) -> Result<U, E>,
    ) -> Result<JvmStackFrame<U>, E> {
        fn map_entry<V, U, E>(
            entry: &Entry<V>,
            map: &mut dyn FnMut(&V) -> Result<U, E>,
        ) -> Result<Entry<U>, E> {
            Ok(match entry {
                Entry::Value(value) => Entry::Value(map(value)?),
                Entry::Top => Entry::Top,
                Entry::UninitializedLocal => Entry::UninitializedLocal,
                Entry::OutOfScope => Entry::OutOfScope,
            })
        }

        Ok(JvmStackFrame {
            max_stack: self.max_stack,
            local_variables: self
                .local_variables
                .iter()
                .map(|entry| map_entry(entry, &mut map))
                .collect::<Result<_, _>>()?,
            operand_stack: self
                .operand_stack
                .iter()
                .map(|entry| map_entry(entry, &mut map))
                .collect::<Result<_, _>>()?,
        })
    }

    pub(crate) fn local_variables(&self) -> &[Entry<V>] {
        &self.local_variables
    }

    pub(crate) fn operand_stack(&self) -> &[Entry<V>] {
        &self.operand_stack
    }

    pub(crate) fn invalidate_values_at(
        &mut self,
        local_indices: impl IntoIterator<Item = usize>,
        stack_indices: impl IntoIterator<Item = usize>,
    ) {
        for index in local_indices {
            if let Some(entry) = self.local_variables.get_mut(index) {
                *entry = Entry::UninitializedLocal;
            }
        }
        for index in stack_indices {
            if let Some(entry) = self.operand_stack.get_mut(index) {
                *entry = Entry::UninitializedLocal;
            }
        }
    }
}

fn create_local_variable_entries<V: Clone>(
    desc: &MethodDescriptor,
    max_locals: u16,
    this_value: Option<V>,
    parameters: &[V],
) -> Result<Box<[Entry<V>]>, ExecutionError> {
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
        return Err(ExecutionError::LocalLimitExceed);
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
    let local_variables = this_arg
        .into_iter()
        .chain(args)
        .pad_using(max_locals.into(), |_| Entry::UninitializedLocal)
        .collect();
    Ok(local_variables)
}
