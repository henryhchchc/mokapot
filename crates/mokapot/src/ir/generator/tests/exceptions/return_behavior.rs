use super::*;
use crate::jvm::method;

#[test]
fn synchronized_return_reports_handler_frame_overflow_at_return() {
    let mut method = method(
        [
            (4, InstructionData::Return),
            (10, InstructionData::AStore0),
            (11, InstructionData::Return),
        ],
        "()V",
        vec![handler(4.into()..5.into(), 10.into(), None)],
    );
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    method.body.as_mut().expect("method has a body").max_stack = 0;

    let failure = build_failure(&method);
    assert_eq!(
        failure,
        (Some(4.into()), MokaIRBuildErrorKind::StackOverflow)
    );
}

#[test]
fn synchronized_return_has_only_exceptional_successors() {
    let illegal = cls_name("java/lang/IllegalMonitorStateException");
    let body = [
        (0, InstructionData::IConst1),
        (1, InstructionData::IReturn),
        (10, InstructionData::AStore0),
        (11, InstructionData::IConst0),
        (12, InstructionData::IReturn),
    ];
    let table = vec![handler(
        1.into()..2.into(),
        10.into(),
        Some(illegal.clone()),
    )];
    let mut method = method(body, "()I", table);
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&method).unwrap();
    let Terminator::Return { exceptional, .. } = terminator_at(&ir, 1.into()) else {
        panic!("expected a fallible return terminator");
    };

    assert_eq!(exceptional.len(), 2);
    assert!(
        matches!(exceptional[0].transfer(), Some(ControlTransfer::Exception(Some(it))) if it == &illegal)
    );
    assert_eq!(exceptional[1].transfer(), None);
    assert!(
        exceptional
            .iter()
            .all(|s| !matches!(s.transfer(), Some(ControlTransfer::Unconditional)))
    );
    let pad = ir.block(exceptional[0].block_target().unwrap()).unwrap();
    assert!(matches!(pad.kind, BlockKind::LandingPad { .. }));
}

/// A return that may unwind has exactly one outcome: the method-unwind exit.
#[test]
fn returns_without_a_handler_unwind_conservatively() {
    let mut synchronized = method([(0, InstructionData::Return)], "()V", vec![]);
    synchronized.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&synchronized).unwrap();
    let ret_term = terminator_at(&ir, 0.into());
    assert!(matches!(ret_term, Terminator::Return { value: None, .. }));
    let mut outcomes = ret_term.successors();
    let unwind = outcomes.next().expect("the return has one outcome");
    assert_eq!(unwind.transfer(), None);
    assert_eq!(unwind.block_target(), None);
    assert!(outcomes.next().is_none());
}
