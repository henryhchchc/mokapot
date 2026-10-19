use super::*;
#[test]
fn invalid_stack_shuffles_report_the_source_instruction() {
    let body = [(0, InstructionData::Dup), (1, InstructionData::Return)];
    let underflow = build_failure(&method(body, "()V", vec![]));
    assert_matches!(underflow, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 0.into());

    let body = [
        (0, InstructionData::LLoad0),
        (1, InstructionData::Dup),
        (2, InstructionData::Return),
    ];
    let layout = build_failure(&method(body, "(J)V", vec![]));
    assert_matches!(layout, (Some(pc), MokaIRBuildErrorKind::InvalidSlotLayout) if pc == 1.into());
}

#[test]
fn instance_invocations_require_a_receiver() {
    let method_ref = method_ref("accept", "(IJ)V");
    let mut pool = ConstantPool::new();
    let target = pool.insert_method_ref(method_ref);
    for invocation in [
        InstructionData::InvokeVirtual(target),
        InstructionData::InvokeInterface(target, 4),
        InstructionData::InvokeSpecial(target),
    ] {
        let body = [
            (0, InstructionData::ILoad0),
            (1, InstructionData::LLoad1),
            (2, invocation),
            (3, InstructionData::Return),
        ];
        let method = method_with_pool(pool.clone(), body, "(IJ)V", vec![]);
        let failure = build_failure(&method);
        assert_matches!(failure, (Some(pc), MokaIRBuildErrorKind::StackUnderflow) if pc == 2.into());
    }
}
