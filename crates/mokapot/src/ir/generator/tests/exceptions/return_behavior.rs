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
    let return_terminator = terminator_at(&ir, 1.into());
    let successors = return_terminator.successors();

    assert_eq!(successors.len(), 2);
    assert!(
        matches!(successors[0].transfer(), ControlTransfer::Exception(Some(caught)) if caught == &illegal_monitor_state)
    );
    assert!(matches!(successors[1].transfer(), ControlTransfer::Unwind));
    assert!(
        successors
            .iter()
            .all(|successor| !matches!(successor.transfer(), ControlTransfer::Unconditional))
    );
    assert!(ir.caught_exception(successors[0].target()).is_some());
}

#[test]
fn unhandled_synchronized_return_reaches_unwind() {
    let mut method = method([(0, Instruction::Return)], "()V", vec![]);
    method.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let ir = build(&method).unwrap();
    let return_terminator = terminator_at(&ir, 0.into());

    assert_eq!(return_terminator.kind(), &TerminatorKind::Return(None));
    assert_eq!(return_terminator.successors().len(), 1);
    assert!(matches!(
        return_terminator.successors()[0].transfer(),
        ControlTransfer::Unwind
    ));
    assert_eq!(
        ir.block(return_terminator.successors()[0].target())
            .unwrap()
            .terminator
            .kind(),
        &TerminatorKind::Unwind
    );
}

#[test]
fn explicit_monitor_operations_have_fallible_returns() {
    let method = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::MonitorEnter),
            (2, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![],
    );
    let ir = build(&method).unwrap();
    let return_terminator = terminator_at(&ir, 2.into());
    assert_eq!(return_terminator.successors().len(), 1);
    assert!(matches!(
        return_terminator.successors()[0].transfer(),
        ControlTransfer::Unwind
    ));
}

#[test]
fn monitor_free_nonsynchronized_return_is_conservatively_fallible() {
    let method = method([(0, Instruction::Return)], "()V", vec![]);
    let ir = build(&method).unwrap();
    let return_terminator = terminator_at(&ir, 0.into());
    assert_eq!(return_terminator.successors().len(), 1);
    assert!(matches!(
        return_terminator.successors()[0].transfer(),
        ControlTransfer::Unwind
    ));
}
