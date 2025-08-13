#[cfg(test)]
use proptest::prelude::*;
use crate::ir::{
    generator::{jvm_frame::{JvmStackFrame, SINGLE_SLOT, DUAL_SLOT}, ExecutionError},
    Identifier, LocalValue, Operand,
};

#[test]
fn args_locals_checking() {
    let desc = "[ID)I".parse().unwrap();
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
    fn slot_mismatch(values in any::<Operand>()) {
        let mut stack_frame = JvmStackFrame::new(
            true,
            &"()V".parse().expect("Invalid method desc"),
            0,
            2,
        ).unwrap();
        stack_frame.push_value::<DUAL_SLOT>(values.clone()).unwrap();
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
            (values.len() + values.len().div_ceil(2)).try_into().unwrap(),
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
}