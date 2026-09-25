use std::iter;

use super::*;
use crate::{
    ir::{
        MalformedControlFlow::{InvalidTableSwitchRange, MissingFallthrough, MissingInstruction},
        UnsupportedBytecode::LegacySubroutine,
    },
    jvm::code::WideInstruction,
};

#[test]
fn reports_empty_code_at_the_method_boundary() {
    let method = method(iter::empty::<(u16, Instruction)>(), "()V", vec![]);
    assert_matches!(build(&method), Err(MokaIRBuildError::MissingOrEmptyBody));
}

#[test]
fn reports_a_missing_jump_target_at_that_target() {
    let method = method([(0, Instruction::Goto(10.into()))], "()V", vec![]);
    assert_matches!(
        build(&method),
        Err(MokaIRBuildError::ControlFlow(MissingInstruction(pc))) if pc == 10.into()
    );
}

#[test]
fn rejects_tableswitch_cases_outside_i32_range() {
    let switch = Instruction::TableSwitch {
        low: i32::MAX,
        jump_targets: vec![10.into(), 10.into()],
        default: 10.into(),
    };
    let instructions = [
        (0, Instruction::ILoad0),
        (1, switch),
        (10, Instruction::Return),
    ];
    let method = method(instructions, "(I)V", vec![]);
    assert_matches!(
        build(&method),
        Err(MokaIRBuildError::ControlFlow(InvalidTableSwitchRange(pc))) if pc == 1.into()
    );
}

#[test]
fn rejects_tableswitch_without_cases() {
    let switch = Instruction::TableSwitch {
        low: 0,
        jump_targets: vec![],
        default: 10.into(),
    };
    let instructions = [
        (0, Instruction::ILoad0),
        (1, switch),
        (10, Instruction::Return),
    ];
    let method = method(instructions, "(I)V", vec![]);
    assert_matches!(
        build(&method),
        Err(MokaIRBuildError::ControlFlow(InvalidTableSwitchRange(pc))) if pc == 1.into()
    );
}

#[test]
fn rejects_a_missing_structural_target_even_when_its_source_is_unreachable() {
    let method = method(
        [(0, Instruction::Return), (1, Instruction::Goto(10.into()))],
        "()V",
        vec![],
    );
    assert_matches!(
        build(&method),
        Err(MokaIRBuildError::ControlFlow(MissingInstruction(pc))) if pc == 10.into()
    );
}

#[test]
fn reports_frame_sources_at_the_executed_instruction() {
    let method = method([(3, Instruction::IReturn)], "()I", vec![]);
    let error = frame_failure(&method);
    assert_matches!(error, (Some(pc), MokaIRFrameError::StackUnderflow) if pc == 3.into());
}

#[test]
fn reports_a_missing_fallthrough_at_the_source_instruction() {
    let fallthrough = method([(4, Instruction::Nop)], "()V", vec![]);
    assert_matches!(build(&fallthrough), Err(MokaIRBuildError::ControlFlow(MissingFallthrough(pc))) if pc == 4.into());

    // Structural validation runs off the reachable path too.
    let unreachable = method(
        [(0, Instruction::Return), (1, Instruction::Nop)],
        "()V",
        vec![],
    );
    assert_matches!(build(&unreachable), Err(MokaIRBuildError::ControlFlow(MissingFallthrough(pc))) if pc == 1.into());
}

#[test]
fn reports_method_entry_frame_initialization_failures() {
    let mut method = method([(0, Instruction::Return)], "(I)V", vec![]);
    method.body.as_mut().expect("method has a body").max_locals = 0;
    let error = frame_failure(&method);
    assert_matches!(error, (None, MokaIRFrameError::LocalIndexOutOfBounds));
}

#[test]
fn rejects_every_legacy_subroutine_instruction_even_when_unreachable() {
    let instructions = [
        Instruction::Jsr(0.into()),
        Instruction::JsrW(0.into()),
        Instruction::Ret(0),
        Instruction::Wide(WideInstruction::Ret(300)),
    ];
    for instruction in instructions {
        let method = method([(0, Instruction::Return), (1, instruction)], "()V", vec![]);
        assert_eq!(unsupported(&method), (1.into(), LegacySubroutine));
    }
}
