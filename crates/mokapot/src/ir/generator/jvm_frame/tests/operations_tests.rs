#[cfg(test)]
use proptest::prelude::*;
use crate::ir::{
    generator::{jvm_frame::{JvmStackFrame, SINGLE_SLOT}, ExecutionError},
    Identifier, LocalValue, Operand,
};

proptest! {
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