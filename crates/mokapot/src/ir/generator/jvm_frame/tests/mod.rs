mod operations_tests;
mod stack_frame_tests;

use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::generator::jvm_frame::{ExecutionError, JvmStackFrame},
    types::method_descriptor::MethodDescriptor,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, proptest_derive::Arbitrary)]
struct TestValue(u32);

impl JoinSemiLattice for TestValue {
    fn join_assign(&mut self, other: Self) -> bool {
        if *self >= other {
            false
        } else {
            *self = other;
            true
        }
    }
}

fn frame(
    is_static: bool,
    descriptor: &MethodDescriptor,
    max_locals: u16,
    max_stack: u16,
) -> Result<JvmStackFrame<TestValue>, ExecutionError> {
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
    JvmStackFrame::with_inputs(descriptor, max_locals, max_stack, this_value, &parameters)
}
