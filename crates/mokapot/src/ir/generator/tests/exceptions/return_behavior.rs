use super::*;
use crate::{ir::BlockKind, jvm::method};

#[test]
fn synchronized_return_has_only_exceptional_successors() {
    let illegal = cls_r("java/lang/IllegalMonitorStateException");
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

/// A return that may unwind — in a `synchronized` method, after a monitor operation, or because
/// the JVM permits any return to unwind — has exactly one outcome: the method-unwind exit.
#[test]
fn returns_without_a_handler_unwind_conservatively() {
    let mut synchronized = method([(0, Instruction::Return)], "()V", vec![]);
    synchronized.access_flags |= method::AccessFlags::SYNCHRONIZED;
    let monitor = method(
        [
            (0, Instruction::ALoad0),
            (1, Instruction::MonitorEnter),
            (2, Instruction::Return),
        ],
        "(Ljava/lang/Object;)V",
        vec![],
    );
    let monitor_free = method([(0, Instruction::Return)], "()V", vec![]);

    let cases: [(&str, Method, ProgramCounter); 3] = [
        ("a synchronized method", synchronized, 0.into()),
        ("an explicit monitor operation", monitor, 2.into()),
        ("a monitor-free method", monitor_free, 0.into()),
    ];
    for (label, case, return_pc) in cases {
        let ir = build(&case).unwrap();
        let ret_term = terminator_at(&ir, return_pc);
        assert!(
            matches!(ret_term, Terminator::TryReturn { value: None, .. }),
            "{label} must return fallibly"
        );
        let mut outcomes = ret_term.successors();
        let unwind = outcomes.next().expect("the return has one outcome");
        assert_eq!(unwind.transfer(), None, "{label}");
        assert_eq!(unwind.block_target(), None, "{label}");
        assert!(
            outcomes.next().is_none(),
            "{label} has more than one outcome"
        );
    }
}
