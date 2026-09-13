use super::*;
use crate::jvm::code::WideInstruction;

fn block_with_origin(method: &MokaIRMethod, pc: ProgramCounter) -> &BasicBlock {
    let id = method
        .source_map()
        .instructions_at(pc)
        .next()
        .expect("the source PC must survive subroutine expansion");
    method
        .blocks()
        .find(|block| {
            block.terminator().id() == id
                || block
                    .operations()
                    .iter()
                    .any(|instruction| instruction.id() == id)
        })
        .expect("the mapped instruction must belong to a block")
}

#[test]
fn expands_a_basic_jsr_ret_pair_to_gotos() {
    let ir = build(&method(
        [
            (0, Instruction::Jsr(10.into())),
            (3, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    ))
    .unwrap();

    let call = block_with_origin(&ir, 0.into()).terminator();
    let ret = block_with_origin(&ir, 11.into()).terminator();
    assert_eq!(call.kind(), &TerminatorKind::Goto);
    assert_eq!(ret.kind(), &TerminatorKind::Goto);
    assert_eq!(
        ret.successors()[0].target(),
        block_with_origin(&ir, 3.into()).id()
    );
}

#[test]
fn clones_a_shared_subroutine_per_call_context_with_shared_provenance() {
    let ir = build(&method(
        [
            (0, Instruction::Jsr(20.into())),
            (3, Instruction::Jsr(20.into())),
            (6, Instruction::Return),
            (20, Instruction::AStore0),
            (21, Instruction::IConst1),
            (22, Instruction::Pop),
            (23, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    ))
    .unwrap();

    assert_eq!(ir.source_map().instructions_at(21.into()).count(), 2);
    assert_eq!(ir.source_map().instructions_at(23.into()).count(), 2);
    let continuations = ir
        .source_map()
        .instructions_at(23.into())
        .map(|id| {
            ir.blocks()
                .find(|block| block.terminator().id() == id)
                .unwrap()
                .terminator()
                .successors()[0]
                .target()
        })
        .collect::<HashSet<_>>();
    assert_eq!(
        continuations,
        HashSet::from([
            block_with_origin(&ir, 3.into()).id(),
            block_with_origin(&ir, 6.into()).id(),
        ])
    );
}

#[test]
fn supports_nested_subroutines_and_returns_to_an_ancestor() {
    let ir = build(&method(
        [
            (0, Instruction::Jsr(20.into())),
            (3, Instruction::Return),
            (20, Instruction::AStore0),
            (21, Instruction::Jsr(30.into())),
            (24, Instruction::Ret(0)),
            (30, Instruction::AStore1),
            (31, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    ))
    .unwrap();

    assert_eq!(
        block_with_origin(&ir, 31.into()).terminator().successors()[0].target(),
        block_with_origin(&ir, 3.into()).id()
    );
    assert_eq!(ir.source_map().instructions_at(24.into()).count(), 0);
}

#[test]
fn expands_jsr_w_and_wide_ret() {
    let ir = method(
        [
            (0, Instruction::JsrW(10.into())),
            (5, Instruction::Return),
            (10, Instruction::Wide(WideInstruction::AStore(300))),
            (11, Instruction::Wide(WideInstruction::Ret(300))),
        ],
        "()V",
        vec![],
    );
    let mut ir = ir;
    ir.body.as_mut().unwrap().max_locals = 301;
    let ir = build(&ir).unwrap();

    assert_eq!(
        block_with_origin(&ir, 0.into()).terminator().kind(),
        &TerminatorKind::Goto
    );
    assert_eq!(
        block_with_origin(&ir, 11.into()).terminator().kind(),
        &TerminatorKind::Goto
    );
}

#[test]
fn gives_each_context_its_own_handler_entry_and_caught_value() {
    let ir = build(&method(
        [
            (0, Instruction::Jsr(20.into())),
            (3, Instruction::Jsr(20.into())),
            (6, Instruction::Return),
            (20, Instruction::AStore0),
            (21, Instruction::AConstNull),
            (
                22,
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (23, Instruction::Pop),
            (24, Instruction::Ret(0)),
            (30, Instruction::AStore1),
            (31, Instruction::Goto(24.into())),
        ],
        "()V",
        vec![ExceptionTableEntry {
            covered_pc: 22.into()..23.into(),
            handler_pc: 30.into(),
            catch_type: Some("java/lang/Throwable".parse().unwrap()),
        }],
    ))
    .unwrap();

    let handler_values = ir
        .blocks()
        .filter_map(|block| {
            ir.caught_exception(block.id())
                .map(|value| (block.id(), value))
        })
        .collect::<Vec<_>>();
    assert_eq!(handler_values.len(), 2);
    assert_ne!(handler_values[0].1, handler_values[1].1);
    assert!(handler_values.iter().all(|&(block, value)| {
        ir.definition_of(value) == Some(ValueDefinition::CaughtException(block))
            && ir
                .source_map()
                .origins_of(ir.block(block).unwrap().terminator().id())
                .count()
                == 0
    }));
    assert_eq!(ir.source_map().instructions_at(31.into()).count(), 2);
}

#[test]
fn rejects_recursive_and_root_level_legacy_returns() {
    let recursive = method(
        [
            (0, Instruction::Jsr(10.into())),
            (3, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::Jsr(10.into())),
            (14, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    );
    assert!(matches!(
        build(&recursive),
        Err(MokaIRBuildError::MalformedControlFlow)
    ));

    let root_ret = method([(0, Instruction::Ret(0))], "()V", vec![]);
    assert!(matches!(
        build(&root_ret),
        Err(MokaIRBuildError::FrameError(_) | MokaIRBuildError::MalformedControlFlow)
    ));

    let multiple_returns = method(
        [
            (0, Instruction::Jsr(10.into())),
            (3, Instruction::Return),
            (10, Instruction::AStore0),
            (11, Instruction::IConst0),
            (12, Instruction::IfEq(20.into())),
            (15, Instruction::Ret(0)),
            (20, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    );
    assert!(matches!(
        build(&multiple_returns),
        Err(MokaIRBuildError::MalformedControlFlow)
    ));
}
