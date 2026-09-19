use super::*;
use crate::jvm::method;

#[test]
fn synchronized_return_has_only_exceptional_successors() {
    let illegal_monitor_state: ClassRef = "java/lang/IllegalMonitorStateException".parse().unwrap();
    let mut method = method(
        [
            (0, Instruction::IConst1),
            (1, Instruction::IReturn),
            (10, Instruction::AStore0),
            (11, Instruction::IConst0),
            (12, Instruction::IReturn),
        ],
        "()I",
        vec![ExceptionTableEntry {
            covered_pc: 1.into()..2.into(),
            handler_pc: 10.into(),
            catch_type: Some(illegal_monitor_state.clone()),
        }],
    );
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&method).unwrap();
    let ret_term = terminator_at(&ir, 1.into());
    let Terminator::TryReturn { exceptional, .. } = ret_term else {
        panic!("expected a fallible return terminator");
    };

    assert_eq!(exceptional.len(), 2);
    assert!(
        matches!(exceptional[0].transfer(), ControlTransfer::Exception(Some(caught)) if caught == &illegal_monitor_state)
    );
    assert!(matches!(exceptional[1].transfer(), ControlTransfer::Unwind));
    assert!(
        exceptional
            .iter()
            .all(|successor| !matches!(successor.transfer(), ControlTransfer::Unconditional))
    );
    assert!(matches!(
        ir.block(exceptional[0].block_target().unwrap())
            .unwrap()
            .kind,
        crate::ir::BlockKind::LandingPad { .. }
    ));
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
    assert!(matches!(
        ret_term.successors().next().unwrap().transfer(),
        ControlTransfer::Unwind
    ));
    assert_eq!(
        ret_term.successors().next().unwrap().target(),
        crate::ir::SuccessorTarget::Unwind
    );
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
    assert!(matches!(
        ret_term.successors().next().unwrap().transfer(),
        ControlTransfer::Unwind
    ));
}

#[test]
fn monitor_free_nonsynchronized_return_is_conservatively_fallible() {
    let method = method([(0, Instruction::Return)], "()V", vec![]);
    let ir = build(&method).unwrap();
    let ret_term = terminator_at(&ir, 0.into());
    assert_eq!(ret_term.successors().count(), 1);
    assert!(matches!(
        ret_term.successors().next().unwrap().transfer(),
        ControlTransfer::Unwind
    ));
}
