use std::iter::{once, repeat_n};

use ValueCategory::{Category1, Category2};
use itertools::Itertools;

use super::error::Error;
use crate::{
    intrinsics::see_jvm_spec,
    ir::ValueId,
    types::{field_type::ValueCategory, method_descriptor::MethodDescriptor},
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum StackOperation {
    Pop,
    Pop2,
    Dup,
    DupX1,
    DupX2,
    Dup2,
    Dup2X1,
    Dup2X2,
    Swap,
}

impl StackOperation {
    const fn consumed_slots(self) -> usize {
        match self {
            Self::Pop | Self::Dup => 1,
            Self::Pop2 | Self::DupX1 | Self::Dup2 | Self::Swap => 2,
            Self::DupX2 | Self::Dup2X1 => 3,
            Self::Dup2X2 => 4,
        }
    }

    fn matching_form(self, categories: &[ValueCategory]) -> Option<(usize, &'static [usize])> {
        let form: (usize, &'static [usize]) = match self {
            Self::Pop if categories.ends_with(&[Category1]) => (1, &[]),
            Self::Pop2 if categories.ends_with(&[Category2]) => (1, &[]),
            Self::Pop2 if categories.ends_with(&[Category1, Category1]) => (2, &[]),
            Self::Dup if categories.ends_with(&[Category1]) => (1, &[0, 0][..]),
            Self::DupX1 if categories.ends_with(&[Category1, Category1]) => (2, &[1, 0, 1][..]),
            Self::DupX2 if categories.ends_with(&[Category2, Category1]) => (2, &[1, 0, 1][..]),
            Self::DupX2 if categories.ends_with(&[Category1, Category1, Category1]) => {
                (3, &[2, 0, 1, 2][..])
            }
            Self::Dup2 if categories.ends_with(&[Category2]) => (1, &[0, 0][..]),
            Self::Dup2 if categories.ends_with(&[Category1, Category1]) => (2, &[0, 1, 0, 1][..]),
            Self::Dup2X1 if categories.ends_with(&[Category1, Category2]) => (2, &[1, 0, 1][..]),
            Self::Dup2X1 if categories.ends_with(&[Category1, Category1, Category1]) => {
                (3, &[1, 2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category2, Category2]) => (2, &[1, 0, 1][..]),
            Self::Dup2X2 if categories.ends_with(&[Category1, Category1, Category2]) => {
                (3, &[2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category2, Category1, Category1]) => {
                (3, &[1, 2, 0, 1, 2][..])
            }
            Self::Dup2X2 if categories.ends_with(&[Category1, Category1, Category1, Category1]) => {
                (4, &[2, 3, 0, 1, 2, 3][..])
            }
            Self::Swap if categories.ends_with(&[Category1, Category1]) => (2, &[1, 0][..]),
            _ => return None,
        };
        Some(form)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
struct StackItem {
    value: ValueId,
    category: ValueCategory,
}

#[doc = see_jvm_spec!(2, 6, 2)]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub(crate) struct OperandStack {
    max_slots: u16,
    slot_count: usize,
    values: Vec<StackItem>,
}

impl OperandStack {
    pub(super) fn with_max_slots(max_slots: u16) -> Self {
        Self {
            max_slots,
            slot_count: 0,
            values: Vec::with_capacity(max_slots.into()),
        }
    }

    pub(crate) fn push(&mut self, value: ValueId, category: ValueCategory) -> Result<(), Error> {
        let slot_count = self.slot_count + category.slot_count();
        if slot_count > usize::from(self.max_slots) {
            return Err(Error::StackOverflow);
        }
        self.values.push(StackItem { value, category });
        self.slot_count = slot_count;
        Ok(())
    }

    pub(crate) fn pop(&mut self, expected: ValueCategory) -> Result<ValueId, Error> {
        let top = self.values.last().ok_or(Error::StackUnderflow)?;
        if top.category != expected {
            return Err(Error::InvalidSlotLayout);
        }
        let value = top.value;
        self.values.truncate(self.values.len() - 1);
        self.slot_count -= expected.slot_count();
        Ok(value)
    }

    pub(crate) fn pop_arguments(
        &mut self,
        descriptor: &MethodDescriptor,
    ) -> Result<Vec<ValueId>, Error> {
        let mut arguments: Vec<_> = descriptor
            .parameters_types
            .iter()
            .rev()
            .map(|value_type| self.pop(value_type.value_category()))
            .try_collect()?;
        arguments.reverse();
        Ok(arguments)
    }

    pub(crate) fn apply(&mut self, operation: StackOperation) -> Result<(), Error> {
        if self.slot_count < operation.consumed_slots() {
            return Err(Error::StackUnderflow);
        }
        let categories = self
            .values
            .iter()
            .map(|value| value.category)
            .collect::<Vec<_>>();
        let (input_len, output_indices) = operation
            .matching_form(&categories)
            .ok_or(Error::InvalidSlotLayout)?;
        let input_start = self.values.len() - input_len;
        let replaced = &self.values[input_start..];
        let output_slots = output_indices
            .iter()
            .map(|&index| replaced[index].category.slot_count())
            .sum::<usize>();
        let resulting_slots = self.slot_count - operation.consumed_slots() + output_slots;
        if resulting_slots > usize::from(self.max_slots) {
            return Err(Error::StackOverflow);
        }

        let output = output_indices
            .iter()
            .map(|&index| replaced[index])
            .collect::<Vec<_>>();
        self.values.truncate(input_start);
        self.values.extend(output);
        self.slot_count = resulting_slots;
        Ok(())
    }

    pub(super) const fn max_slots(&self) -> u16 {
        self.max_slots
    }

    pub(super) fn has_same_shape(&self, other: &Self) -> bool {
        self.max_slots == other.max_slots
            && self
                .values
                .iter()
                .map(|value| value.category)
                .eq(other.values.iter().map(|value| value.category))
    }

    pub(super) fn merge_from_with(
        &mut self,
        other: Self,
        mut merge_values: impl FnMut(usize, &mut ValueId, ValueId),
    ) {
        let mut slot = 0;
        for (lhs, rhs) in self.values.iter_mut().zip(other.values) {
            slot += lhs.category.slot_count() - 1;
            merge_values(slot, &mut lhs.value, rhs.value);
            slot += 1;
        }
    }

    pub(super) fn single_value(&self, expected: ValueCategory) -> Result<&ValueId, Error> {
        let [value] = self.values.as_slice() else {
            return Err(Error::InvalidSlotLayout);
        };
        if value.category != expected {
            return Err(Error::InvalidSlotLayout);
        }
        Ok(&value.value)
    }

    pub(super) fn slot_values(&self) -> impl Iterator<Item = Option<&ValueId>> {
        self.values.iter().flat_map(|it| {
            repeat_n(None, it.category.slot_count() - 1).chain(once(Some(&it.value)))
        })
    }
}

#[cfg(test)]
mod tests {
    use ValueCategory::{Category1, Category2};
    use proptest::prelude::*;

    use super::{Error, OperandStack, StackItem, StackOperation};
    use crate::{
        intrinsics::see_jvm_spec,
        ir::{IdAllocator, ValueId},
        types::{field_type::ValueCategory, method_descriptor::MethodDescriptor},
    };

    /// The operand stack depths the forms of `operation` transform.
    #[doc = see_jvm_spec!(6, 5)]
    const fn depths(operation: StackOperation) -> (usize, usize) {
        match operation {
            StackOperation::Pop => (1, 0),
            StackOperation::Pop2 => (2, 0),
            StackOperation::Dup => (1, 2),
            StackOperation::DupX1 => (2, 3),
            StackOperation::DupX2 => (3, 4),
            StackOperation::Dup2 => (2, 4),
            StackOperation::Dup2X1 => (3, 5),
            StackOperation::Dup2X2 => (4, 6),
            StackOperation::Swap => (2, 2),
        }
    }

    /// The most operands any instruction replaces: `dup2_x2` form 1 replaces four.
    const MAX_REPLACED_OPERANDS: usize = 4;

    fn slots(items: &[StackItem]) -> usize {
        items.iter().map(|item| item.category.slot_count()).sum()
    }

    /// A stack holding `items`, with room for `extra_slots` slots beyond them.
    fn stack_with(items: &[StackItem], extra_slots: usize) -> OperandStack {
        let room =
            u16::try_from(slots(items) + extra_slots).expect("the test stacks fit in u16 slots");
        let mut stack = OperandStack::with_max_slots(room);

        for item in items {
            stack
                .push(item.value, item.category)
                .expect("the stack has room for its own operands");
        }
        stack
    }

    /// Applies `operation` to `input` and reads `expected` back through
    /// [`OperandStack::pop`]; both lists are bottom-to-top.
    #[doc = see_jvm_spec!(6, 5)]
    fn assert_reads_back(
        operation: StackOperation,
        input: &[(ValueId, ValueCategory)],
        expected: &[(ValueId, ValueCategory)],
    ) {
        let items: Vec<StackItem> = input
            .iter()
            .map(|&(value, category)| StackItem { value, category })
            .collect();
        let mut stack = stack_with(&items, 2);
        stack
            .apply(operation)
            .expect("the operands fit a form of the instruction");
        for &(value, category) in expected.iter().rev() {
            assert_eq!(
                stack.pop(category),
                Ok(value),
                "{operation:?} did not leave {expected:?}",
            );
        }
        assert_eq!(stack.slot_count, 0, "{operation:?} left operands behind");
    }

    proptest! {
        /// An instruction transforms the stack by the documented depths, or
        /// rejects it untouched.
        #[doc = see_jvm_spec!(6, 5)]
        #[test]
        fn an_operation_moves_the_documented_depth(
            operation in any::<StackOperation>(),
            items in prop::collection::vec(any::<StackItem>(), 0..6),
            extra_slots in 0..=4_usize,
        ) {
            let mut stack = stack_with(&items, extra_slots);
            let before = stack.clone();
            let operands = stack.values.clone();
            let depth_before = stack.slot_count;

            match stack.apply(operation) {
                Ok(()) => {
                    let (replaced, pushed) = depths(operation);
                    prop_assert_eq!(stack.slot_count + replaced, depth_before + pushed);
                    for item in &stack.values {
                        prop_assert!(operands.contains(item), "{:?} was not on the stack", item);
                    }
                    let untouched = operands.len().saturating_sub(MAX_REPLACED_OPERANDS);
                    prop_assert!(
                        stack.values.len() >= untouched,
                        "{operation:?} replaced more than {MAX_REPLACED_OPERANDS} operands",
                    );
                    prop_assert_eq!(&stack.values[..untouched], &operands[..untouched]);
                }
                Err(error) => {
                    use Error::{StackUnderflow, InvalidSlotLayout, StackOverflow};
                    prop_assert!(
                        matches!(&error, StackUnderflow | InvalidSlotLayout | StackOverflow),
                        "{operation:?} failed with {error:?}"
                    );
                    prop_assert_eq!(&stack, &before, "a rejected operation changed the stack");
                }
            }
            // The cached depth is always the recomputed width of the operands it counts.
            prop_assert_eq!(stack.slot_count, slots(&stack.values));
        }

        /// `push` accepts an operand while the stack has room for its slots and
        /// refuses it otherwise; a category 2 operand costs two slots.
        #[doc = see_jvm_spec!(2, 6, 2)]
        #[test]
        fn push_checks_the_slot_budget(
            items in prop::collection::vec(any::<StackItem>(), 0..6),
            extra_slots in 0..=2_usize,
            category in any::<ValueCategory>(),
            value in any::<ValueId>(),
        ) {
            let mut stack = stack_with(&items, extra_slots);
            let before = stack.clone();
            let pushed = stack.push(value, category);

            if category.slot_count() <= extra_slots {
                prop_assert_eq!(pushed, Ok(()));
                prop_assert_eq!(stack.values.last(), Some(&StackItem { value, category }));
                prop_assert_eq!(stack.slot_count, slots(&items) + category.slot_count());
            } else {
                prop_assert_eq!(pushed, Err(Error::StackOverflow));
                prop_assert_eq!(&stack, &before, "a rejected push changed the stack");
            }
            prop_assert_eq!(stack.slot_count, slots(&stack.values));
        }
    }

    /// `pop_arguments` consumes arguments pushed in declaration order and returns
    /// them in that order, leaving the operands below them, as `invokevirtual` requires.
    #[doc = see_jvm_spec!(6, 5)]
    #[test]
    fn pop_arguments_pops_in_reverse_declaration_order() {
        // `(JI)V`: the last argument takes two slots, the one before it takes one.
        let descriptor: MethodDescriptor = "(JI)V".parse().expect("the descriptor is valid");
        let mut ids = IdAllocator::default();
        let arguments = [(ids.new_id(), Category2), (ids.new_id(), Category1)]
            .map(|(value, category)| StackItem { value, category });
        let below = [StackItem {
            value: ids.new_id(),
            category: Category1,
        }];
        let pushed: Vec<StackItem> = below.iter().chain(&arguments).copied().collect();
        let mut stack = stack_with(&pushed, 1);

        let expected: Vec<ValueId> = arguments.iter().map(|item| item.value).collect();
        assert_eq!(expected.len(), descriptor.parameters_types.len());
        assert_eq!(stack.pop_arguments(&descriptor), Ok(expected));
        assert_eq!(stack.values.as_slice(), below.as_slice());
        assert_eq!(stack.slot_count, slots(&stack.values));

        // `(IJ)V`: the last argument takes two slots, and the stack's top holds one.
        let mismatched: MethodDescriptor = "(IJ)V".parse().expect("the descriptor is valid");
        let short = [
            StackItem {
                value: ids.new_id(),
                category: Category1,
            },
            StackItem {
                value: ids.new_id(),
                category: Category1,
            },
        ];
        let mut stack = stack_with(&short, 2);
        let before = stack.clone();
        assert_eq!(
            stack.pop_arguments(&mismatched),
            Err(Error::InvalidSlotLayout),
        );
        assert_eq!(stack, before, "a rejected pop changed the stack");
    }

    /// The operand order the order-sensitive instructions leave behind, read back
    /// through `pop` (the only order-observable accessor); one form each.
    #[doc = see_jvm_spec!(6, 5)]
    #[test]
    fn an_operation_leaves_its_operands_in_the_documented_order() {
        use StackOperation::*;
        let mut ids = IdAllocator::default();
        let (a, b, c, d) = (ids.new_id(), ids.new_id(), ids.new_id(), ids.new_id());
        let c1 = |value: ValueId| (value, Category1);
        let c2 = |value: ValueId| (value, Category2);

        assert_reads_back(Dup, &[c1(a)], &[c1(a), c1(a)]);
        assert_reads_back(DupX1, &[c1(a), c1(c)], &[c1(c), c1(a), c1(c)]);
        assert_reads_back(DupX2, &[c2(b), c1(a)], &[c1(a), c2(b), c1(a)]);
        assert_reads_back(Dup2, &[c1(a), c1(c)], &[c1(a), c1(c), c1(a), c1(c)]);
        assert_reads_back(Dup2, &[c2(b)], &[c2(b), c2(b)]);
        assert_reads_back(Dup2X1, &[c1(a), c2(b)], &[c2(b), c1(a), c2(b)]);
        assert_reads_back(
            Dup2X2,
            &[c1(a), c1(b), c1(c), c1(d)],
            &[c1(c), c1(d), c1(a), c1(b), c1(c), c1(d)],
        );
        assert_reads_back(Swap, &[c1(a), c1(c)], &[c1(c), c1(a)]);
    }
}
