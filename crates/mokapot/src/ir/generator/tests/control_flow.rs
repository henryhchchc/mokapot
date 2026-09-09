#[allow(
    clippy::wildcard_imports,
    reason = "generator tests share the fixture helpers from their parent module"
)]
use super::*;

#[test]
fn switch_retains_parallel_successor_arms() {
    let method = method(
        [
            (0.into(), Instruction::ILoad0),
            (
                1.into(),
                Instruction::LookupSwitch {
                    default: 10.into(),
                    match_targets: BTreeMap::from([(1, 10.into()), (2, 10.into())]),
                },
            ),
            (10.into(), Instruction::Return),
        ],
        "(I)V",
        vec![],
    );
    let ir = method.brew().unwrap();
    let switch = ir.block(ir.entry_block()).unwrap().terminator();

    assert!(matches!(switch.kind(), TerminatorKind::Switch { .. }));
    assert_eq!(switch.successors().len(), 3);
    assert_eq!(
        switch
            .successors()
            .iter()
            .map(Successor::id)
            .collect::<HashSet<_>>()
            .len(),
        3
    );
    assert!(
        switch
            .successors()
            .windows(2)
            .all(|pair| pair[0].target() == pair[1].target())
    );
    assert_eq!(ir.control_flow_graph().edges().count(), 3);
}

#[test]
fn fallible_exit_keeps_normal_then_ordered_handler_arms() {
    let exception_table = vec![
        ExceptionTableEntry {
            covered_pc: 0.into()..1.into(),
            handler_pc: 3.into(),
            catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
        },
        ExceptionTableEntry {
            covered_pc: 0.into()..1.into(),
            handler_pc: 2.into(),
            catch_type: Some("java/lang/Exception".parse().unwrap()),
        },
    ];
    let method = method(
        [
            (0.into(), Instruction::IConst0),
            (1.into(), Instruction::Return),
            (2.into(), Instruction::Return),
            (3.into(), Instruction::Return),
        ],
        "()V",
        exception_table,
    );
    let ir = method.brew().unwrap();
    let fallible = ir.block(ir.entry_block()).unwrap().terminator();

    assert_eq!(fallible.kind(), &TerminatorKind::Fallible);
    assert_eq!(ir.source_map().origins_of(fallible.id()).count(), 0);
    assert!(matches!(
        fallible.successors()[0].transfer(),
        ControlTransfer::Unconditional
    ));
    let handler_types = fallible.successors()[1..]
        .iter()
        .map(|successor| match successor.transfer() {
            ControlTransfer::Exception(types) => types.iter().next().unwrap().0.as_ref(),
            _ => unreachable!(),
        })
        .collect::<Vec<_>>();
    assert_eq!(
        handler_types,
        ["java/lang/RuntimeException", "java/lang/Exception"]
    );
}

#[test]
fn normally_reachable_handler_still_starts_a_block() {
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (1.into(), Instruction::AStore1),
            (2.into(), Instruction::Return),
        ],
        "(Ljava/lang/Throwable;)V",
        vec![ExceptionTableEntry {
            covered_pc: 0.into()..1.into(),
            handler_pc: 1.into(),
            catch_type: Some("java/lang/Throwable".parse().unwrap()),
        }],
    );
    let ir = method.brew().unwrap();
    let entry = ir.block(ir.entry_block()).unwrap();
    let handler = entry.terminator().successors()[0].target();

    assert_eq!(ir.blocks().len(), 2);
    assert_ne!(handler, ir.entry_block());
    assert_eq!(ir.block(handler).unwrap().instructions().len(), 0);
    assert!(ir.caught_exception(handler).is_some());
}

#[test]
fn throw_is_a_source_backed_terminator() {
    let method = method(
        [
            (0.into(), Instruction::ALoad0),
            (1.into(), Instruction::AThrow),
        ],
        "(Ljava/lang/Throwable;)V",
        vec![],
    );
    let ir = method.brew().unwrap();
    let block = ir.block(ir.entry_block()).unwrap();

    assert!(matches!(
        block.terminator().kind(),
        TerminatorKind::Throw(_)
    ));
    assert_eq!(
        ir.source_map()
            .origins_of(block.terminator().id())
            .collect::<Vec<_>>(),
        [ProgramCounter::from(1)]
    );
}
