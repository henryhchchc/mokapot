use std::iter;

use super::*;
use crate::{
    ir::{
        MalformedBytecode::{MissingEntry, MissingFallthrough, MissingInstruction},
        UnsupportedBytecode::LegacySubroutine,
    },
    jvm::code::WideInstruction,
};

#[test]
fn reports_empty_code_at_the_method_boundary() {
    let method = method(iter::empty::<(u16, Instruction)>(), "()V", vec![]);
    assert_eq!(malformed(&method), (None, MissingEntry));
}

#[test]
fn reports_a_missing_jump_target_at_that_target() {
    let method = method([(0, Instruction::Goto(10.into()))], "()V", vec![]);
    assert_eq!(malformed(&method), (Some(10.into()), MissingInstruction));
}

#[test]
fn rejects_a_missing_structural_target_even_when_its_source_is_unreachable() {
    let method = method(
        [(0, Instruction::Return), (1, Instruction::Goto(10.into()))],
        "()V",
        vec![],
    );
    assert_eq!(malformed(&method), (Some(10.into()), MissingInstruction));
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
    assert_eq!(
        malformed(&fallthrough),
        (Some(4.into()), MissingFallthrough)
    );

    // Structural validation runs off the reachable path too.
    let unreachable = method(
        [(0, Instruction::Return), (1, Instruction::Nop)],
        "()V",
        vec![],
    );
    assert_eq!(
        malformed(&unreachable),
        (Some(1.into()), MissingFallthrough)
    );
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
