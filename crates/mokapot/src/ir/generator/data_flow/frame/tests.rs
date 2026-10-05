use proptest::prelude::*;

use super::*;
use crate::{
    ir::{IdAllocator, test::prelude::entry_values},
    types::{field_type::FieldType, method_descriptor::ReturnType},
};

/// A local variable table of `slot_count` variables holding `items` from variable zero.
fn locals_with(items: &[(ValueId, ValueCategory)], slot_count: u16) -> LocalVariables {
    let entry = MethodDescriptor {
        parameters_types: Vec::new(),
        return_type: ReturnType::Void,
    };
    let mut locals = LocalVariables::for_method_entry(&entry, slot_count, None, &[])
        .expect("an empty entry layout always fits");
    let mut index = 0_u16;
    for &(value, category) in items {
        locals
            .set(index, value, category)
            .expect("consecutive items fit by construction");
        index += u16::try_from(category.slot_count())
            .expect("a category occupies two variables at most");
    }
    locals
}

/// A frame whose locals hold `items` from variable zero, sized `slot_count`, with an operand stack
/// of one fresh identity per category and `extra_slots` slots of room.
fn frame_with(
    items: &[(ValueId, ValueCategory)],
    slot_count: u16,
    categories: &[ValueCategory],
    extra_slots: usize,
    ids: &mut IdAllocator<ValueId>,
) -> Frame {
    let operands: Vec<(ValueId, ValueCategory)> = categories
        .iter()
        .map(|&category| (ids.new_id(), category))
        .collect();
    let width: usize = operands
        .iter()
        .map(|&(_, category)| category.slot_count())
        .sum();
    let room = u16::try_from(width + extra_slots).expect("the test stacks fit in u16 slots");
    let mut stack = OperandStack::with_max_slots(room);
    for (value, category) in operands {
        stack
            .push(value, category)
            .expect("the stack has room for its own operands");
    }
    Frame {
        locals: locals_with(items, slot_count),
        stack,
    }
}

/// The local variable table length the merge tests use.
const MERGE_LOCALS: u16 = 8;

/// The operand stack room the entry-frame tests pass.
const ENTRY_MAX_STACK: u16 = 4;

proptest! {
    #[test]
    fn entry_frame_assembles_locals_and_operand_stack(
        parameter_types in prop::collection::vec(any::<FieldType>(), 0..8),
        instance in any::<bool>(),
    ) {
        let descriptor = MethodDescriptor {
            parameters_types: parameter_types,
            return_type: ReturnType::Void,
        };
        let (receiver, parameters) = entry_values(instance, descriptor.parameters_types.len());
        let max_locals = u16::try_from(
            usize::from(instance)
                + descriptor.parameters_types.iter()
                    .map(|value_type| value_type.value_category().slot_count())
                    .sum::<usize>(),
        ).expect("a descriptor fits in u16 slots");
        let frame = Frame::for_method_entry(
            &descriptor,
            max_locals,
            ENTRY_MAX_STACK,
            receiver,
            &parameters,
        )
        .expect("the entry values fit their own slot count");

        let expected_locals = LocalVariables::for_method_entry(
            &descriptor,
            max_locals,
            receiver,
            &parameters,
        )
        .expect("the locals fit");
        let expected_stack = OperandStack::with_max_slots(ENTRY_MAX_STACK);
        prop_assert_eq!(frame.locals, expected_locals);
        prop_assert_eq!(frame.stack, expected_stack);
    }

    /// A merge visits every value of the merged frame once, reads the receiving frame's own value,
    /// and leaves a frame merged with itself unchanged.
    #[doc = see_jvm_spec!(4, 10, 2, 2)]
    #[test]
    fn merging_frames_visits_every_surviving_value_once_and_is_idempotent(
        categories in prop::collection::vec(any::<ValueCategory>(), 0..5),
        lhs_locals in prop::collection::vec((any::<ValueId>(), any::<ValueCategory>()), 0..5),
        rhs_locals in prop::collection::vec((any::<ValueId>(), any::<ValueCategory>()), 0..5),
    ) {
        let mut ids = IdAllocator::default();
        let mut lhs = frame_with(&lhs_locals, MERGE_LOCALS, &categories, 1, &mut ids);
        let rhs = frame_with(&rhs_locals, MERGE_LOCALS, &categories, 1, &mut ids);

        let mut identical = lhs.clone();
        identical
            .merge_from_with(lhs.clone(), |_, lhs, rhs| *lhs = rhs)
            .expect("a frame has the same shape as itself");
        prop_assert_eq!(&identical, &lhs, "merging a frame with itself changed it");

        let before = lhs.clone();
        let mut visited: Vec<(Position, ValueId, ValueId)> = Vec::new();
        lhs.merge_from_with(rhs, |position, lhs, rhs| {
            visited.push((position, *lhs, rhs));
            *lhs = rhs;
        })
        .expect("frames of one shape merge");

        let mut positions: Vec<Position> = visited.iter().map(|&(position, _, _)| position).collect();
        positions.sort_unstable();
        let mut unique = positions.clone();
        unique.dedup();
        prop_assert_eq!(unique.len(), positions.len(), "a position was visited twice");
        for &(position, value, joined) in &visited {
            prop_assert_eq!(
                before.value_at(position),
                Some(&value),
                "the merge did not read the receiving frame's value at {:?}",
                position
            );
            prop_assert_eq!(
                lhs.value_at(position),
                Some(&joined),
                "the merged frame did not store the joined value at {:?}",
                position
            );
        }

        // A position holds a value in the merged frame exactly when the callback visited it.
        let locals = (0..MERGE_LOCALS).map(|slot| Position::Local(usize::from(slot)));
        let stack_slots: usize = categories
            .iter()
            .map(|&category| category.slot_count())
            .sum();
        let stack = (0..stack_slots).map(Position::Stack);
        for position in locals.chain(stack) {
            prop_assert_eq!(
                lhs.value_at(position).is_some(),
                positions.contains(&position),
                "the merged frame and the visited positions disagree at {:?}",
                position,
            );
        }
    }

    /// Frames whose local variable counts or operand stack shapes differ cannot
    /// merge, and a rejected merge changes nothing.
    #[test]
    fn merge_from_with_rejects_incompatible_shapes(
        categories in prop::collection::vec(any::<ValueCategory>(), 0..5),
        lhs_locals in prop::collection::vec((any::<ValueId>(), any::<ValueCategory>()), 0..5),
        rhs_locals in prop::collection::vec((any::<ValueId>(), any::<ValueCategory>()), 0..5),
        perturbation in 0..3_usize,
    ) {
        let mut ids = IdAllocator::default();
        let mut lhs = frame_with(&lhs_locals, MERGE_LOCALS, &categories, 1, &mut ids);
        let rhs = match perturbation {
            // A local variable more than the receiving frame has.
            0 => frame_with(&rhs_locals, MERGE_LOCALS + 1, &categories, 1, &mut ids),
            // A slot of operand stack room more.
            1 => frame_with(&rhs_locals, MERGE_LOCALS, &categories, 2, &mut ids),
            // An operand more.
            _ => {
                let mut longer = categories.clone();
                longer.push(ValueCategory::Category1);
                frame_with(&rhs_locals, MERGE_LOCALS, &longer, 1, &mut ids)
            }
        };
        let before = lhs.clone();

        let mut visited = Vec::new();
        let merged = lhs.merge_from_with(rhs, |position, _, _| visited.push(position));
        prop_assert_eq!(merged, Err(FrameError::IncompatibleFrameShape));
        prop_assert!(visited.is_empty(), "a rejected merge visited a position");
        prop_assert_eq!(lhs, before, "a rejected merge changed the frame");
    }
}
