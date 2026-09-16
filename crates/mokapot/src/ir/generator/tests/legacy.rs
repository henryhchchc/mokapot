use super::*;
use crate::{
    ir::{
        BlockId,
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, PathValue},
        },
        expression::{Condition, Expression},
    },
    jvm::code::WideInstruction,
};

fn block_with_origin(method: &MokaIRMethod, pc: ProgramCounter) -> &BasicBlock {
    let id = method
        .source_map()
        .instructions_at(pc)
        .next()
        .expect("the source PC must survive lowering");
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

fn terminator_with_origin(method: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .map(BasicBlock::terminator)
                .find(|terminator| terminator.id() == id)
        })
        .expect("the source PC must map to a terminator")
}

fn subroutine_target(call: &Terminator) -> BlockId {
    let [successor] = call.successors() else {
        panic!("a jsr must have exactly one subroutine-call arm");
    };
    match successor.transfer() {
        ControlTransfer::SubroutineCall { .. } => successor.target(),
        transfer => panic!("expected a subroutine-call arm, got {transfer:?}"),
    }
}

fn terminator_block(method: &MokaIRMethod, terminator: &Terminator) -> BlockId {
    method
        .blocks()
        .find(|block| block.terminator().id() == terminator.id())
        .map(BasicBlock::id)
        .expect("an emitted terminator must belong to a block")
}

fn return_address_at(
    method: &MokaIRMethod,
    pc: ProgramCounter,
    continuation: ProgramCounter,
) -> ValueId {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .flat_map(BasicBlock::operations)
                .find(|operation| operation.id() == id)
        })
        .and_then(|operation| match operation.kind() {
            OperationKind::Definition {
                value,
                expr: Expression::ReturnAddress(actual_continuation),
            } if *actual_continuation == continuation => Some(*value),
            _ => None,
        })
        .expect("a jsr must define its return-address token")
}

#[test]
fn retains_a_jsr_ret_pair_as_public_subroutine_control_flow() {
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

    let call = terminator_with_origin(&ir, 0.into());
    let token = return_address_at(&ir, 0.into(), 3.into());
    assert_eq!(call.kind(), &TerminatorKind::SubroutineCall);
    assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
    assert!(
        matches!(call.successors(), [successor] if matches!(successor.transfer(), ControlTransfer::SubroutineCall { continuation } if *continuation == 3.into()))
    );

    let ret = terminator_with_origin(&ir, 11.into());
    assert_eq!(
        ret.kind(),
        &TerminatorKind::SubroutineReturn { address: token }
    );
    let [successor] = ret.successors() else {
        panic!("a single caller must produce one return arm");
    };
    assert_eq!(subroutine_target(call), terminator_block(&ir, ret));
    assert_eq!(successor.target(), block_with_origin(&ir, 3.into()).id());
    assert_eq!(
        successor.transfer(),
        &ControlTransfer::SubroutineReturn {
            continuation: 3.into(),
            guard: BranchGuard::of(BooleanVariable::Positive(Condition::Equal(
                PathValue::Variable(token),
                PathValue::ReturnAddress(3.into()),
            ))),
        }
    );
}

#[test]
fn shares_one_subroutine_body_and_dispatches_each_continuation_by_token() {
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

    assert_eq!(ir.source_map().instructions_at(21.into()).count(), 1);
    assert_eq!(ir.source_map().instructions_at(23.into()).count(), 1);
    assert_eq!(ir.source_map().instructions_at(20.into()).count(), 0);
    let first_subroutine_target = subroutine_target(terminator_with_origin(&ir, 0.into()));
    let second_subroutine_target = subroutine_target(terminator_with_origin(&ir, 3.into()));
    assert_eq!(first_subroutine_target, second_subroutine_target);
    assert!(matches!(
        ir.block(first_subroutine_target)
            .expect("a subroutine-call target must be emitted")
            .terminator()
            .kind(),
        TerminatorKind::SubroutineReturn { .. }
    ));
    let ret = terminator_with_origin(&ir, 23.into());
    let TerminatorKind::SubroutineReturn { address } = ret.kind() else {
        panic!("ret must materialize as a subroutine return");
    };
    let continuations = ret
        .successors()
        .iter()
        .map(|successor| match successor.transfer() {
            ControlTransfer::SubroutineReturn {
                continuation,
                guard,
            } => {
                assert_eq!(
                    guard,
                    &BranchGuard::of(BooleanVariable::Positive(Condition::Equal(
                        PathValue::Variable(*address),
                        PathValue::ReturnAddress(*continuation),
                    )))
                );
                (*continuation, successor.target())
            }
            transfer => panic!("expected a subroutine return arm, got {transfer:?}"),
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        continuations,
        BTreeMap::from([
            (3.into(), block_with_origin(&ir, 3.into()).id()),
            (6.into(), block_with_origin(&ir, 6.into()).id())
        ])
    );
}

#[test]
fn accepts_recursive_and_multiple_return_subroutines() {
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
    assert!(build(&recursive).is_ok());

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
    let ir = build(&multiple_returns).expect("multiple ret sites must share the same activation");
    assert_eq!(ir.source_map().instructions_at(15.into()).count(), 1);
    assert_eq!(ir.source_map().instructions_at(20.into()).count(), 1);
}

#[test]
fn supports_jsr_w_and_wide_ret() {
    let mut method = method(
        [
            (0, Instruction::JsrW(10.into())),
            (5, Instruction::Return),
            (10, Instruction::Wide(WideInstruction::AStore(300))),
            (11, Instruction::Wide(WideInstruction::Ret(300))),
        ],
        "()V",
        vec![],
    );
    method.body.as_mut().unwrap().max_locals = 301;
    let ir = build(&method).unwrap();

    assert_eq!(ir.source_map().instructions_at(10.into()).count(), 0);
    let token = return_address_at(&ir, 0.into(), 5.into());
    assert_eq!(
        terminator_with_origin(&ir, 0.into()).kind(),
        &TerminatorKind::SubroutineCall
    );
    assert!(matches!(
        terminator_with_origin(&ir, 11.into()).kind(),
        TerminatorKind::SubroutineReturn { address } if *address == token
    ));
}

#[test]
fn shares_handler_entry_and_source_provenance_across_subroutine_callers() {
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
    assert_eq!(handler_values.len(), 1);
    assert_eq!(
        ir.definition_of(handler_values[0].1),
        Some(ValueDefinition::CaughtException(handler_values[0].0))
    );
    assert_eq!(ir.source_map().instructions_at(31.into()).count(), 1);
}

#[test]
fn rejects_root_and_scalar_legacy_returns() {
    let root_ret = method([(0, Instruction::Ret(0))], "()V", vec![]);
    assert!(build(&root_ret).is_err());

    let scalar_ret = method(
        [
            (0, Instruction::IConst0),
            (1, Instruction::IStore0),
            (2, Instruction::Ret(0)),
        ],
        "()V",
        vec![],
    );
    assert!(
        matches!(build(&scalar_ret), Err(MokaIRBuildError::MalformedBytecode { pc: Some(pc), kind: crate::ir::MalformedBytecode::InvalidSubroutineReturn }) if pc == 2.into())
    );
}
