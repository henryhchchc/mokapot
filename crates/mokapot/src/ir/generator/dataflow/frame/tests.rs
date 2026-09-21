use std::iter::repeat_n;

use proptest::prelude::*;

use super::{Frame, FrameError, Position};
use crate::{
    ir::{IdAllocator, ValueId},
    tests::arb_field_type,
    types::{
        Descriptor,
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
    },
};

fn entry_values(instance: bool, parameter_count: usize) -> (Option<ValueId>, Vec<ValueId>) {
    let mut allocator = IdAllocator::default();
    let receiver = instance.then(|| allocator.new_id());
    let parameters = (0..parameter_count).map(|_| allocator.new_id()).collect();
    (receiver, parameters)
}

fn slot_width(value_type: &FieldType) -> usize {
    match value_type.descriptor().as_str() {
        "J" | "D" => 2,
        _ => 1,
    }
}

proptest! {
    #[test]
    fn entry_frame_lays_out_entry_values(
        parameter_types in prop::collection::vec(arb_field_type(), 0..8),
        instance in any::<bool>(),
    ) {
        let descriptor = MethodDescriptor {
            parameters_types: parameter_types,
            return_type: ReturnType::Void,
        };
        let (receiver, parameters) = entry_values(instance, descriptor.parameters_types.len());
        let entries: Vec<&ValueId> = receiver.iter().chain(&parameters).collect();

        let mut layout: Vec<Option<usize>> = Vec::new();
        let mut entry = 0;
        if instance {
            layout.push(Some(entry));
            entry += 1;
        }
        for value_type in &descriptor.parameters_types {
            layout.push(Some(entry));
            entry += 1;
            layout.extend(repeat_n(None, slot_width(value_type) - 1));
        }

        let max_locals = u16::try_from(layout.len()).expect("a descriptor must fit in u16 slots");
        let frame = Frame::for_method_entry(&descriptor, max_locals, 4, receiver, &parameters)
            .expect("the entry values must fit in their own slot count");

        let frame_type = if instance { "instance" } else { "static" };
        for (slot, expected) in layout.iter().enumerate() {
            let actual = frame.value_at(Position::Local(slot));
            let expected = expected.map(|index| entries[index]);
            prop_assert_eq!(actual, expected, "slot {} of the {} entry frame", slot, frame_type);
        }

        if max_locals > 0 {
            let error_frame = Frame::for_method_entry(&descriptor, max_locals - 1, 4, receiver, &parameters);
            prop_assert!(
                matches!(error_frame, Err(FrameError::LocalIndexOutOfBounds)),
                "a local table one slot short must be rejected",
            );
        }
    }
}
