use crate::ir::generator::{
    JvmFrameError,
    jvm::frame::{CATEGORY_1, CATEGORY_2},
};
#[cfg(test)]
use proptest::prelude::*;

use super::{TestValue, frame};

#[test]
fn args_locals_checking() {
    let desc = "([ID)I".parse().unwrap();
    let too_small_locals = frame(false, &desc, 2, 2);
    assert!(too_small_locals.is_err());
    let correct = frame(false, &desc, 4, 2);
    assert!(correct.is_ok());
}

#[test]
#[should_panic(expected = "assertion `left == right` failed")]
fn joining_frames_with_different_stack_capacities_panics() {
    let desc = "()V".parse().expect("valid descriptor");
    let mut lhs = frame(true, &desc, 0, 1).expect("valid frame");
    let rhs = frame(true, &desc, 0, 2).expect("valid frame");

    lhs.merge_from_with(rhs, |_, _, _| false);
}

proptest! {
    #[test]
    fn push_pop(args in prop::collection::vec(any::<TestValue>(), 0..10)) {
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            args.len().try_into().unwrap(),
        ).unwrap();
        for arg in &args {
            stack_frame.push_value::<CATEGORY_1>(*arg).expect("Fail to push");
        }
        for arg in args.iter().rev() {
            let popped = stack_frame.pop_value::<CATEGORY_1>().expect("Fail to pop");
            assert_eq!(popped, arg.clone());
        }
    }

    #[test]
    fn push_pop_dual_slot(args in prop::collection::vec(any::<TestValue>(), 0..10)) {
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            (args.len() * 2).try_into().unwrap(),
        ).unwrap();
        for arg in &args {
            stack_frame.push_value::<CATEGORY_2>(*arg).expect("Fail to push");
        }
        for arg in args.iter().rev() {
            let popped = stack_frame.pop_value::<CATEGORY_2>().expect("Fail to pop");
            assert_eq!(popped, arg.clone());
        }
    }

    #[test]
    fn overflow(push_count in 10u16..20, capacity in 0u16..10) {
        prop_assume!(push_count > capacity);
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            capacity,
        ).unwrap();
        for i in 0..push_count {
            let value = TestValue(u32::from(i));
            if i < capacity {
                stack_frame.push_value::<CATEGORY_1>(value).expect("Fail to push");
            } else {
                assert!(matches!(
                    stack_frame.push_value::<CATEGORY_1>(value),
                    Err(JvmFrameError::StackOverflow),
                ));
            }
        }
    }

    #[test]
    fn underflow(push_count in 0u16..10, pop_count in 10u16..20) {
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            push_count,
        ).unwrap();
        for i in 0..push_count {
            let value = TestValue(u32::from(i));
            stack_frame.push_value::<CATEGORY_1>(value).expect("Fail to push");
        }
        for _ in 0..push_count {
            stack_frame.pop_value::<CATEGORY_1>().expect("Fail to pop");
        }
        for _ in push_count..pop_count {
            assert!(matches!(
                stack_frame.pop_value::<CATEGORY_1>(),
                Err(JvmFrameError::StackUnderflow),
            ));
        }
    }

    #[test]
    fn slot_mismatch(values in any::<TestValue>()) {
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            2,
        ).unwrap();
        stack_frame.push_value::<CATEGORY_2>(values).unwrap();
        stack_frame.pop_value::<CATEGORY_1>().unwrap();
        assert!(matches!(
            stack_frame.pop_value::<CATEGORY_1>(),
            Err(JvmFrameError::InvalidSlotLayout),
        ));
    }

    #[test]
    fn mixed_width_values(values in prop::collection::vec(any::<TestValue>(), 0..10)) {
        let mut stack_frame = frame(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            (values.len() + values.len().div_ceil(2)).try_into().unwrap(),
        ).unwrap();
        for (i, value) in values.iter().enumerate() {
            if i % 2 == 0 {
                stack_frame.push_value::<CATEGORY_2>(*value).expect("Fail to push");
            } else {
                stack_frame.push_value::<CATEGORY_1>(*value).expect("Fail to push");
            }
        }
        for (i, value) in values.iter().enumerate().rev() {
            let popped = if i % 2 == 0 {
                stack_frame.pop_value::<CATEGORY_2>().expect("Fail to pop")
            } else {
                stack_frame.pop_value::<CATEGORY_1>().expect("Fail to pop")
            };
            assert_eq!(popped, value.clone());
        }
    }
}
