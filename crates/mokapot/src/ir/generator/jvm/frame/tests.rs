use crate::{
    ir::generator::jvm::frame::{
        Frame, JvmFrameError, StackOperation,
        ValueCategory::{self, Category1, Category2},
    },
    types::method_descriptor::MethodDescriptor,
};
use proptest::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, proptest_derive::Arbitrary)]
struct TestValue(u32);

fn frame(
    is_static: bool,
    descriptor: &MethodDescriptor,
    max_locals: u16,
    max_stack: u16,
) -> Result<Frame<TestValue>, JvmFrameError> {
    let this_value = (!is_static).then_some(TestValue(0));
    let parameter_offset = u32::from(!is_static);
    let parameters = descriptor
        .parameters_types
        .iter()
        .enumerate()
        .map(|(index, _)| {
            let index = u32::try_from(index).expect("descriptor parameter count fits u32");
            TestValue(index + parameter_offset)
        })
        .collect::<Vec<_>>();
    Frame::for_method_entry(descriptor, max_locals, max_stack, this_value, &parameters)
}

fn assert_stack(frame: &mut Frame<TestValue>, expected_top_first: &[(TestValue, ValueCategory)]) {
    for (expected_value, category) in expected_top_first {
        assert_eq!(
            frame.stack.pop(*category).expect("stack value"),
            *expected_value
        );
    }
    assert!(matches!(
        frame.stack.pop(Category1),
        Err(JvmFrameError::StackUnderflow)
    ));
}

#[test]
fn method_entry_initializes_parameters_and_checks_local_capacity() {
    let descriptor = "([ID)V".parse().expect("valid descriptor");

    assert!(matches!(
        frame(false, &descriptor, 3, 0),
        Err(JvmFrameError::LocalIndexOutOfBounds)
    ));

    let frame = frame(false, &descriptor, 4, 0).expect("parameters fit");
    assert_eq!(frame.locals.get(0, Category1).unwrap(), &TestValue(0));
    assert_eq!(frame.locals.get(1, Category1).unwrap(), &TestValue(1));
    assert_eq!(frame.locals.get(2, Category2).unwrap(), &TestValue(2));
}

#[test]
fn merging_incompatible_frame_shapes_returns_an_error() {
    let descriptor = "()V".parse().expect("valid descriptor");
    let mut target = frame(true, &descriptor, 0, 1).expect("valid frame");
    let source = frame(true, &descriptor, 0, 2).expect("valid frame");

    assert!(matches!(
        target.merge_from_with(source, |_, _, _| false),
        Err(JvmFrameError::IncompatibleFrameShape)
    ));
}

#[test]
fn local_access_checks_the_value_category() {
    let descriptor = "(J)V".parse().expect("valid descriptor");
    let mut frame = frame(true, &descriptor, 2, 0).expect("parameters fit");

    assert!(matches!(
        frame.locals.get(0, Category1),
        Err(JvmFrameError::InvalidSlotLayout)
    ));
    frame
        .locals
        .set(0, TestValue(1), Category1)
        .expect("local exists");
    assert_eq!(frame.locals.get(0, Category1).unwrap(), &TestValue(1));
    assert!(matches!(
        frame.locals.get(0, Category2),
        Err(JvmFrameError::InvalidSlotLayout)
    ));
}

#[test]
fn writing_an_upper_slot_invalidates_the_category_2_value() {
    let descriptor = "(J)V".parse().expect("valid descriptor");
    let mut frame = frame(true, &descriptor, 2, 0).expect("parameters fit");

    frame
        .locals
        .set(1, TestValue(1), Category1)
        .expect("local exists");

    assert!(matches!(
        frame.locals.get(0, Category2),
        Err(JvmFrameError::UnavailableLocal)
    ));
    assert_eq!(frame.locals.get(1, Category1).unwrap(), &TestValue(1));
}

type StackContents = &'static [(TestValue, ValueCategory)];

struct StackOperationCase {
    operation: StackOperation,
    input_bottom_first: StackContents,
    expected_top_first: StackContents,
}

const LEGAL_STACK_OPERATION_CASES: &[StackOperationCase] = &[
    StackOperationCase {
        operation: StackOperation::Pop,
        input_bottom_first: &[(TestValue(1), Category1)],
        expected_top_first: &[],
    },
    StackOperationCase {
        operation: StackOperation::Pop2,
        input_bottom_first: &[(TestValue(1), Category2)],
        expected_top_first: &[],
    },
    StackOperationCase {
        operation: StackOperation::Pop2,
        input_bottom_first: &[(TestValue(2), Category1), (TestValue(1), Category1)],
        expected_top_first: &[],
    },
    StackOperationCase {
        operation: StackOperation::Dup,
        input_bottom_first: &[(TestValue(1), Category1)],
        expected_top_first: &[(TestValue(1), Category1), (TestValue(1), Category1)],
    },
    StackOperationCase {
        operation: StackOperation::DupX1,
        input_bottom_first: &[(TestValue(2), Category1), (TestValue(1), Category1)],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::DupX2,
        input_bottom_first: &[(TestValue(2), Category2), (TestValue(1), Category1)],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category2),
            (TestValue(1), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::DupX2,
        input_bottom_first: &[
            (TestValue(3), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
        ],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(3), Category1),
            (TestValue(1), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2,
        input_bottom_first: &[(TestValue(1), Category2)],
        expected_top_first: &[(TestValue(1), Category2), (TestValue(1), Category2)],
    },
    StackOperationCase {
        operation: StackOperation::Dup2,
        input_bottom_first: &[(TestValue(2), Category1), (TestValue(1), Category1)],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
            (TestValue(2), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X1,
        input_bottom_first: &[(TestValue(2), Category1), (TestValue(1), Category2)],
        expected_top_first: &[
            (TestValue(1), Category2),
            (TestValue(2), Category1),
            (TestValue(1), Category2),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X1,
        input_bottom_first: &[
            (TestValue(3), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
        ],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(3), Category1),
            (TestValue(1), Category1),
            (TestValue(2), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X2,
        input_bottom_first: &[(TestValue(2), Category2), (TestValue(1), Category2)],
        expected_top_first: &[
            (TestValue(1), Category2),
            (TestValue(2), Category2),
            (TestValue(1), Category2),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X2,
        input_bottom_first: &[
            (TestValue(3), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category2),
        ],
        expected_top_first: &[
            (TestValue(1), Category2),
            (TestValue(2), Category1),
            (TestValue(3), Category1),
            (TestValue(1), Category2),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X2,
        input_bottom_first: &[
            (TestValue(3), Category2),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
        ],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(3), Category2),
            (TestValue(1), Category1),
            (TestValue(2), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Dup2X2,
        input_bottom_first: &[
            (TestValue(4), Category1),
            (TestValue(3), Category1),
            (TestValue(2), Category1),
            (TestValue(1), Category1),
        ],
        expected_top_first: &[
            (TestValue(1), Category1),
            (TestValue(2), Category1),
            (TestValue(3), Category1),
            (TestValue(4), Category1),
            (TestValue(1), Category1),
            (TestValue(2), Category1),
        ],
    },
    StackOperationCase {
        operation: StackOperation::Swap,
        input_bottom_first: &[(TestValue(2), Category1), (TestValue(1), Category1)],
        expected_top_first: &[(TestValue(2), Category1), (TestValue(1), Category1)],
    },
];

#[test]
fn stack_operations_implement_every_legal_jvm_form() {
    for case in LEGAL_STACK_OPERATION_CASES {
        let mut frame = frame(true, &"()V".parse().unwrap(), 0, 8).unwrap();
        for (value, category) in case.input_bottom_first {
            frame.stack.push(*value, *category).unwrap();
        }
        frame.stack.apply(case.operation).unwrap();
        assert_stack(&mut frame, case.expected_top_first);
    }
}

#[test]
fn invalid_stack_operations_leave_the_stack_unchanged() {
    let cases = [
        StackOperationCase {
            operation: StackOperation::Pop,
            input_bottom_first: &[(TestValue(1), Category2)],
            expected_top_first: &[(TestValue(1), Category2)],
        },
        StackOperationCase {
            operation: StackOperation::Dup,
            input_bottom_first: &[(TestValue(1), Category2)],
            expected_top_first: &[(TestValue(1), Category2)],
        },
        StackOperationCase {
            operation: StackOperation::Swap,
            input_bottom_first: &[(TestValue(2), Category2), (TestValue(1), Category1)],
            expected_top_first: &[(TestValue(1), Category1), (TestValue(2), Category2)],
        },
    ];

    for case in cases {
        let mut frame = frame(true, &"()V".parse().unwrap(), 0, 4).unwrap();
        for (value, category) in case.input_bottom_first {
            frame.stack.push(*value, *category).unwrap();
        }
        assert!(matches!(
            frame.stack.apply(case.operation),
            Err(JvmFrameError::InvalidSlotLayout)
        ));
        assert_stack(&mut frame, case.expected_top_first);
    }
}

#[test]
fn stack_operations_report_underflow() {
    let mut frame = frame(true, &"()V".parse().unwrap(), 0, 8).unwrap();
    for operation in [
        StackOperation::Pop,
        StackOperation::Pop2,
        StackOperation::Dup,
        StackOperation::DupX1,
        StackOperation::DupX2,
        StackOperation::Dup2,
        StackOperation::Dup2X1,
        StackOperation::Dup2X2,
        StackOperation::Swap,
    ] {
        assert!(matches!(
            frame.stack.apply(operation),
            Err(JvmFrameError::StackUnderflow)
        ));
    }
}

proptest! {
    #[test]
    fn operand_stack_preserves_lifo_order_and_categories(
        values in prop::collection::vec(any::<TestValue>(), 0..10),
    ) {
        let capacity = values.len() + values.len().div_ceil(2);
        let mut frame = frame(true, &"()V".parse().unwrap(), 0, capacity.try_into().unwrap()).unwrap();
        for (index, value) in values.iter().enumerate() {
            let category = if index % 2 == 0 { Category2 } else { Category1 };
            frame.stack.push(*value, category).unwrap();
        }
        for (index, value) in values.iter().enumerate().rev() {
            let category = if index % 2 == 0 { Category2 } else { Category1 };
            assert_eq!(frame.stack.pop(category).unwrap(), *value);
        }
    }

    #[test]
    fn failed_pop_does_not_consume_a_value(value in any::<TestValue>()) {
        let mut frame = frame(true, &"()V".parse().unwrap(), 0, 2).unwrap();
        frame.stack.push(value, Category2).unwrap();
        assert!(matches!(
            frame.stack.pop(Category1),
            Err(JvmFrameError::InvalidSlotLayout)
        ));
        assert_eq!(frame.stack.pop(Category2).unwrap(), value);
    }

    #[test]
    fn pushes_cannot_exceed_max_stack(capacity in 0u16..10) {
        let mut frame = frame(true, &"()V".parse().unwrap(), 0, capacity).unwrap();
        for value in 0..capacity {
            frame.stack.push(TestValue(u32::from(value)), Category1).unwrap();
        }
        assert!(matches!(
            frame.stack.push(TestValue(u32::from(capacity)), Category1),
            Err(JvmFrameError::StackOverflow)
        ));
    }
}
