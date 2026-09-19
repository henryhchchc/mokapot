use super::*;
use crate::ir::BlockKind;
use crate::jvm::method;

#[test]
fn synchronized_return_has_only_exceptional_successors() {
    let illegal: ClassRef = "java/lang/IllegalMonitorStateException".parse().unwrap();
    let body = [
        (0, Instruction::IConst1),
        (1, Instruction::IReturn),
        (10, Instruction::AStore0),
        (11, Instruction::IConst0),
        (12, Instruction::IReturn),
    ];
    let table = vec![handler(
        1.into()..2.into(),
        10.into(),
        Some(illegal.clone()),
    )];
    let mut method = method(body, "()I", table);
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&method).unwrap();
    let Terminator::TryReturn { exceptional, .. } = terminator_at(&ir, 1.into()) else {
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

#[test]
fn unhandled_synchronized_return_reaches_unwind() {
    let mut method = method([(0, Instruction::Return)], "()V", vec![]);
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&method).unwrap();
    let ret_term = terminator_at(&ir, 0.into());

    assert!(matches!(
        ret_term,
        Terminator::TryReturn { value: None, .. }
    ));
    assert_eq!(ret_term.successors().count(), 1);
    assert_eq!(ret_term.successors().next().unwrap().transfer(), None);
    assert_eq!(ret_term.successors().next().unwrap().block_target(), None);
}

#[test]
fn explicit_monitor_operations_have_fallible_returns() {
    let instructions = [
        (0, Instruction::ALoad0),
        (1, Instruction::MonitorEnter),
        (2, Instruction::Return),
    ];
    let method = method(instructions, "(Ljava/lang/Object;)V", vec![]);
    let ir = build(&method).unwrap();
    let ret_term = terminator_at(&ir, 2.into());
    assert_eq!(ret_term.successors().count(), 1);
    assert_eq!(ret_term.successors().next().unwrap().transfer(), None);
}

#[test]
fn monitor_free_nonsynchronized_return_is_conservatively_fallible() {
    let method = method([(0, Instruction::Return)], "()V", vec![]);
    let ir = build(&method).unwrap();
    let ret_term = terminator_at(&ir, 0.into());
    assert_eq!(ret_term.successors().count(), 1);
    assert_eq!(ret_term.successors().next().unwrap().transfer(), None);
}
