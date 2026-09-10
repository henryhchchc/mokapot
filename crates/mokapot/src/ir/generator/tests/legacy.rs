#[allow(
    clippy::wildcard_imports,
    reason = "generator tests share the fixture helpers from their parent module"
)]
use super::*;
use crate::jvm::code::WideInstruction;

fn block_with_origin(method: &MokaIRMethod, pc: ProgramCounter) -> &BasicBlock {
    let id = method
        .source_map()
        .instructions_at(pc)
        .next()
        .expect("the source PC must survive normalization");
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
fn normalizes_a_basic_jsr_ret_pair_to_gotos() {
    let ir = build(&method(
        [
            (0.into(), Instruction::Jsr(10.into())),
            (3.into(), Instruction::Return),
            (10.into(), Instruction::AStore0),
            (11.into(), Instruction::Ret(0)),
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
            (0.into(), Instruction::Jsr(20.into())),
            (3.into(), Instruction::Jsr(20.into())),
            (6.into(), Instruction::Return),
            (20.into(), Instruction::AStore0),
            (21.into(), Instruction::IConst1),
            (22.into(), Instruction::Pop),
            (23.into(), Instruction::Ret(0)),
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
            (0.into(), Instruction::Jsr(20.into())),
            (3.into(), Instruction::Return),
            (20.into(), Instruction::AStore0),
            (21.into(), Instruction::Jsr(30.into())),
            (24.into(), Instruction::Ret(0)),
            (30.into(), Instruction::AStore1),
            (31.into(), Instruction::Ret(0)),
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
fn normalizes_jsr_w_and_wide_ret() {
    let ir = method(
        [
            (0.into(), Instruction::JsrW(10.into())),
            (5.into(), Instruction::Return),
            (10.into(), Instruction::Wide(WideInstruction::AStore(300))),
            (11.into(), Instruction::Wide(WideInstruction::Ret(300))),
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
            (0.into(), Instruction::Jsr(20.into())),
            (3.into(), Instruction::Jsr(20.into())),
            (6.into(), Instruction::Return),
            (20.into(), Instruction::AStore0),
            (21.into(), Instruction::AConstNull),
            (
                22.into(),
                Instruction::CheckCast("java/lang/String".parse().unwrap()),
            ),
            (23.into(), Instruction::Pop),
            (24.into(), Instruction::Ret(0)),
            (30.into(), Instruction::AStore1),
            (31.into(), Instruction::Goto(24.into())),
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
            (0.into(), Instruction::Jsr(10.into())),
            (3.into(), Instruction::Return),
            (10.into(), Instruction::AStore0),
            (11.into(), Instruction::Jsr(10.into())),
            (14.into(), Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    );
    assert!(matches!(
        build(&recursive),
        Err(MokaIRBuildError::MalformedControlFlow)
    ));

    let root_ret = method([(0.into(), Instruction::Ret(0))], "()V", vec![]);
    assert!(matches!(
        build(&root_ret),
        Err(MokaIRBuildError::ExecutionError(_) | MokaIRBuildError::MalformedControlFlow)
    ));

    let multiple_returns = method(
        [
            (0.into(), Instruction::Jsr(10.into())),
            (3.into(), Instruction::Return),
            (10.into(), Instruction::AStore0),
            (11.into(), Instruction::IConst0),
            (12.into(), Instruction::IfEq(20.into())),
            (15.into(), Instruction::Ret(0)),
            (20.into(), Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    );
    assert!(matches!(
        build(&multiple_returns),
        Err(MokaIRBuildError::MalformedControlFlow)
    ));
}
