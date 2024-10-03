use std::{collections::BTreeSet, iter::once};

use crate::{
    ir::{Identifier, Operand},
    jvm::code::ProgramCounter,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};
use itertools::Itertools;

pub(super) type SlotWidth = bool;
pub(super) const SINGLE_SLOT: SlotWidth = false;
pub(super) const DUAL_SLOT: SlotWidth = true;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub(super) struct JvmStackFrame {
    max_locals: u16,
    max_stack: u16,
    local_variables: Vec<Entry>,
    operand_stack: Vec<Entry>,
    pub possible_ret_addresses: BTreeSet<ProgramCounter>,
}

#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
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
    ) -> Result<Self, ExecutionError> {
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
                let arg_idx =
                    u16::try_from(arg_idx).expect("The number of args should be within u16");
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
        Ok(Self {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: Vec::with_capacity(max_stack.into()),
            possible_ret_addresses: BTreeSet::new(),
        })
    }

    #[inline]
    fn push_raw(&mut self, value: Entry) -> Result<(), ExecutionError> {
        let stack_size =
            u16::try_from(self.operand_stack.len()).expect("The stack size should be within u16");
        if stack_size >= self.max_stack {
            Err(ExecutionError::StackOverflow)
        } else {
            self.operand_stack.push(value);
            Ok(())
        }
    }

    #[inline]
    fn pop_raw(&mut self) -> Result<Entry, ExecutionError> {
        self.operand_stack
            .pop()
            .ok_or(ExecutionError::StackUnderflow)
    }

    pub(super) fn pop_value<const SLOT: SlotWidth>(&mut self) -> Result<Operand, ExecutionError> {
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

    pub(super) fn push_value<const SLOT: SlotWidth>(
        &mut self,
        value: Operand,
    ) -> Result<(), ExecutionError> {
        if SLOT == DUAL_SLOT {
            self.push_raw(Entry::Top)?;
        }
        self.push_raw(Entry::Value(value))
    }

    pub(super) fn pop_args(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<Operand>, ExecutionError> {
        use FieldType::Base;
        use PrimitiveType::{Double, Long};
        let mut args = Vec::with_capacity(descriptor.parameters_types.len());
        for param_type in descriptor.parameters_types.iter().rev() {
            let arg = if let Base(Long | Double) = param_type {
                self.pop_value::<DUAL_SLOT>()?
            } else {
                self.pop_value::<SINGLE_SLOT>()?
            };
            args.push(arg);
        }
        args.reverse();
        Ok(args)
    }

    pub(super) fn typed_push(
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

    pub(super) fn get_local<const SLOT: SlotWidth>(
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

    pub(super) fn set_local<const SLOT: SlotWidth>(
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
    pub(super) fn pop(&mut self) -> Result<(), ExecutionError> {
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    pub(super) fn pop2(&mut self) -> Result<(), ExecutionError> {
        let _top_element = self.pop_raw()?;
        let _top_element = self.pop_raw()?;
        Ok(())
    }

    pub(super) fn dup(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup_x1(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup_x2(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        let third_element = self.pop_raw()?;
        self.push_raw(top_element.clone())?;
        self.push_raw(third_element)?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup2(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(second_element.clone())?;
        self.push_raw(top_element.clone())?;
        self.push_raw(second_element)?;
        self.push_raw(top_element)?;
        Ok(())
    }

    pub(super) fn dup2_x1(&mut self) -> Result<(), ExecutionError> {
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

    pub(super) fn dup2_x2(&mut self) -> Result<(), ExecutionError> {
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

    pub(super) fn swap(&mut self) -> Result<(), ExecutionError> {
        let top_element = self.pop_raw()?;
        let second_element = self.pop_raw()?;
        self.push_raw(top_element)?;
        self.push_raw(second_element)?;
        Ok(())
    }
}

impl JvmStackFrame {
    pub(super) fn merge(&self, other: Self) -> Result<Self, ExecutionError> {
        if self.max_locals != other.max_locals {
            Err(ExecutionError::LocalLimitMismatch)?;
        }
        debug_assert!(
            self.local_variables.len() == other.local_variables.len(),
            "The size of the local variables does not match"
        );
        if self.operand_stack.len() != other.operand_stack.len() {
            Err(ExecutionError::StackSizeMismatch)?;
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

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, derive_more::Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum Entry {
    Value(Operand),
    #[display("<top>")]
    Top,
    #[display("<uninitialized_local>")]
    UninitializedLocal,
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
}

#[cfg(test)]
mod test {
    use proptest::{prelude::*, proptest};
    use std::collections::BTreeSet;

    use crate::{
        ir::{generator::ExecutionError, Identifier, LocalValue, Operand},
        types::method_descriptor::MethodDescriptor,
    };

    use super::*;

    #[test]
    fn merge_value_ref() {
        let lhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));
        let rhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(1))));

        let result = Entry::merge(lhs, rhs);
        assert_eq!(
            result,
            Entry::Value(Operand::Phi(BTreeSet::from([
                Identifier::Local(LocalValue::new(0)),
                Identifier::Local(LocalValue::new(1))
            ])))
        );
    }

    #[test]
    fn merge_same_value_ref() {
        let lhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));
        let rhs = Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))));

        let result = Entry::merge(lhs, rhs);
        assert_eq!(
            result,
            Entry::Value(Operand::Just(Identifier::Local(LocalValue::new(0))))
        );
    }

    #[test]
    fn args_locals_checking() {
        let desc: MethodDescriptor = "([ID)I".parse().unwrap();
        let too_small_locals = JvmStackFrame::new(false, &desc, 2, 2);
        assert!(too_small_locals.is_err());
        let correct = JvmStackFrame::new(false, &desc, 4, 2);
        assert!(correct.is_ok());
    }

    proptest! {

        #[test]
        fn push_pop(args in prop::collection::vec(any::<Operand>(), 0..10)) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                args.len().try_into().unwrap(),
            ).unwrap();
            for arg in &args {
                stack_frame.push_value::<SINGLE_SLOT>(arg.clone()).expect("Fail to push");
            }
            for arg in args.iter().rev() {
                let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
                assert_eq!(popped, arg.clone());
            }
        }

        #[test]
        fn push_pop_dual_slot(args in prop::collection::vec(any::<Operand>(), 0..10)) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                (args.len() * 2).try_into().unwrap(),
            ).unwrap();
            for arg in &args {
                stack_frame.push_value::<DUAL_SLOT>(arg.clone()).expect("Fail to push");
            }
            for arg in args.iter().rev() {
                let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
                assert_eq!(popped, arg.clone());
            }
        }

        #[test]
        fn overflow(push_count in 10u16..20, capacity in 0u16..10) {
            prop_assume!(push_count > capacity);
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                capacity,
            ).unwrap();
            for i in 0..push_count {
                let value = Operand::Just(Identifier::Local(LocalValue::new(i)));
                if i < capacity {
                    stack_frame.push_value::<SINGLE_SLOT>(value).expect("Fail to push");
                } else {
                    assert!(matches!(
                        stack_frame.push_value::<SINGLE_SLOT>(value),
                        Err(ExecutionError::StackOverflow),
                    ));
                }
            }
        }

        #[test]
        fn underflow(push_count in 0u16..10, pop_count in 10u16..20) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                push_count,
            ).unwrap();
            for i in 0..push_count {
                let value = Operand::Just(Identifier::Local(LocalValue::new(i)));
                stack_frame.push_value::<SINGLE_SLOT>(value).expect("Fail to push");
            }
            for _ in 0..push_count {
                stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            }
            for _ in push_count..pop_count {
                assert!(matches!(
                    stack_frame.pop_value::<SINGLE_SLOT>(),
                    Err(ExecutionError::StackUnderflow),
                ));
            }
        }

        #[test]
        fn slot_mismatch(valus in any::<Operand>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                2,
            ).unwrap();
            stack_frame.push_value::<DUAL_SLOT>(valus.clone()).unwrap();
            stack_frame.pop_value::<SINGLE_SLOT>().unwrap();
            assert!(matches!(
                stack_frame.pop_value::<SINGLE_SLOT>(),
                Err(ExecutionError::ValueMismatch),
            ));
        }

        #[test]
        fn mixed_width_values(values in prop::collection::vec(any::<Operand>(), 0..10)) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                (values.len() + (values.len() + 1) / 2).try_into().unwrap(),
            ).unwrap();
            for (i, value) in values.iter().enumerate() {
                if i % 2 == 0 {
                    stack_frame.push_value::<DUAL_SLOT>(value.clone()).expect("Fail to push");
                } else {
                    stack_frame.push_value::<SINGLE_SLOT>(value.clone()).expect("Fail to push");
                }
            }
            for (i, value) in values.iter().enumerate().rev() {
                let popped = if i % 2 == 0 {
                    stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop")
                } else {
                    stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop")
                };
                assert_eq!(popped, value.clone());
            }
        }

        #[test]
        fn jvm_pop(pop_count in 0u16..10) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                pop_count,
            ).unwrap();
            for i in 0..pop_count {
                let value = Operand::Just(Identifier::Local(LocalValue::new(i)));
                stack_frame.push_value::<SINGLE_SLOT>(value).expect("Fail to push");
            }
            for _ in 0..pop_count {
                stack_frame.pop().expect("Fail to pop");
            }
            assert!(matches!(
                stack_frame.pop(),
                Err(ExecutionError::StackUnderflow),
            ));
        }

        #[test]
        fn jvm_pop2(pop_count in 0u16..10) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                pop_count * 2,
            ).unwrap();
            for i in 0..(pop_count * 2) {
                let value = Operand::Just(Identifier::Local(LocalValue::new(i)));
                stack_frame.push_value::<SINGLE_SLOT>(value).expect("Fail to push");
            }
            for _ in 0..pop_count {
                stack_frame.pop2().expect("Fail to pop");
            }
            assert!(matches!(
                stack_frame.pop2(),
                Err(ExecutionError::StackUnderflow),
            ));
        }

        #[test]
        fn jvm_dup(value in any::<Operand>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                2,
            ).unwrap();
            stack_frame.push_value::<SINGLE_SLOT>(value.clone()).expect("Fail to push");
            stack_frame.dup().expect("Fail to dup");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, value);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, value);
        }

        #[test]
        fn jvm_dup_x1([v1, v2] in any::<[Operand;2]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                3,
            ).unwrap();
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup_x1().expect("Fail to dup_x1");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }

        #[test]
        fn jvm_dup_x2([v1, v2, v3] in any::<[Operand;3]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                4,
            ).unwrap();

            // Form 1:
            //    ..., value3, value2, value1 →
            //    ..., value1, value3, value2, value1
            //    where value1, value2, and value3 are all values of a category 1
            //    computational type (§2.11.1).
            stack_frame.push_value::<SINGLE_SLOT>(v3.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup_x2().expect("Fail to dup_x2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v3);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);

            // Form 2:
            //    ..., value2, value1 →
            //    ..., value1, value2, value1
            //    where value1 is a value of a category 1 computational type and
            //    value2 is a value of a category 2 computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup_x2().expect("Fail to dup_x2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }

        #[test]
        fn jvm_dup2([v1, v2] in any::<[Operand;2]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                4,
            ).unwrap();

            // Form 1:
            //     ..., value2, value1 →
            //     ..., value2, value1, value2, value1
            //     where both value1 and value2 are values of a category 1 computational type
            //     (§2.11.1).
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2().expect("Fail to dup2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);

            // Form 2:
            //     ..., value →
            //     ..., value, value
            //     where value is a value of a category 2 computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2().expect("Fail to dup2");
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }

        #[test]
        fn jvm_dup2_x1([v1, v2, v3] in any::<[Operand;3]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                5,
            ).unwrap();

            // Form 1:
            //     ..., value3, value2, value1 →
            //     ..., value2, value1, value3, value2, value1
            //     where value1, value2, and value3 are all values of a category 1
            //     computational type (§2.11.1).
            stack_frame.push_value::<SINGLE_SLOT>(v3.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x1().expect("Fail to dup2_x1");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v3);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);

            // Form 2:
            //     ..., value2, value1 →
            //     ..., value1, value2, value1
            //     where value1 is a value of a category 1 computational type and
            //     value2 is a value of a category 2 computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x1().expect("Fail to dup2_x1");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }


        #[test]
        fn jvm_dup2_x2([v1, v2, v3, v4] in any::<[Operand;4]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                6,
            ).unwrap();

            // Form 1:
            //     ..., value4, value3, value2, value1 →
            //     ..., value2, value1, value4, value3, value2, value1
            //     where value1, value2, value3, and value4 are all values of a category 1
            //     computational type (§2.11.1).
            stack_frame.push_value::<SINGLE_SLOT>(v4.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v3.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x2().expect("Fail to dup2_x2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v3);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v4);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);

            // Form 2:
            //    ..., value3, value2, value1 →
            //    ..., value1, value3, value2, value1
            //    where value1 and value2 are both values of a category 1
            //    computational type and value3 is a value of a category 2
            //    computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v3.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x2().expect("Fail to dup2_x2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v3);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);


            // Form 3:
            //     ..., value3, value2, value1 →
            //     ..., value1, value3, value2, value1
            //     where value1 and value2 are both values of a category 1 computational type
            //     and value3 is a value of a category 2 computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v3.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x2().expect("Fail to dup2_x2");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v3);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);

            // Form 4:
            //    ..., value2, value1 →
            //    ..., value1, value2, value1
            //    where value1 and value2 are both values of a category 2
            //    computational type (§2.11.1).
            stack_frame.push_value::<DUAL_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<DUAL_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.dup2_x2().expect("Fail to dup2_x2");
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<DUAL_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }

        #[test]
        fn jvm_swap([v1, v2] in any::<[Operand;2]>()) {
            let mut stack_frame = JvmStackFrame::new(
                true,
                &"()V".parse().expect("Invalid method desc"),
                0,
                2,
            ).unwrap();
            stack_frame.push_value::<SINGLE_SLOT>(v2.clone()).expect("Fail to push");
            stack_frame.push_value::<SINGLE_SLOT>(v1.clone()).expect("Fail to push");
            stack_frame.swap().expect("Fail to swap");
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v2);
            let popped = stack_frame.pop_value::<SINGLE_SLOT>().expect("Fail to pop");
            assert_eq!(popped, v1);
        }

    }
}
