use std::{error::Error, iter};

use super::*;
use crate::{
    ir::{
        MokaIRBuildError,
        MokaIRBuildErrorKind::{
            IncompatibleFrameShape, InvalidMultiArrayDimensions, InvalidTableSwitchRange,
            LocalIndexOutOfBounds, MissingFallthrough, MissingInstruction, MissingOrEmptyBody,
            StackUnderflow, UnsupportedLegacySubroutine,
        },
    },
    jvm::code::WideInstruction,
};

#[test]
fn build_error_display_includes_optional_pc_and_has_no_source() {
    let kind = MissingOrEmptyBody;
    let without_pc = MokaIRBuildError { kind, pc: None };
    assert_eq!(without_pc.to_string(), kind.to_string());
    assert!(without_pc.source().is_none());

    let pc = 7.into();
    let with_pc = MokaIRBuildError {
        kind: StackUnderflow,
        pc: Some(pc),
    };
    assert_eq!(with_pc.to_string(), format!("{StackUnderflow} at {pc}"));
    assert!(with_pc.source().is_none());
}

#[test]
fn reports_empty_code_at_the_method_boundary() {
    let method = method(iter::empty::<(u16, Instruction)>(), "()V", vec![]);
    assert_matches!(build(&method), Err(error) if error.kind == MissingOrEmptyBody && error.pc.is_none());
}

#[test]
fn reports_a_missing_jump_target_at_that_target() {
    let method = method([(0, Instruction::Goto(10.into()))], "()V", vec![]);
    assert_matches!(
        build(&method),
        Err(error) if error.kind == MissingInstruction && error.pc == Some(10.into())
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
        Err(error) if error.kind == InvalidTableSwitchRange && error.pc == Some(1.into())
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
        Err(error) if error.kind == InvalidTableSwitchRange && error.pc == Some(1.into())
    );
}

#[test]
fn rejects_invalid_multianewarray_dimensions() {
    for (array_type, dimensions) in [("[[I", 0), ("[I", 2), ("java/lang/Object", 1)] {
        let instructions = [
            (0, Instruction::Return),
            (
                1,
                Instruction::MultiANewArray(ref_t(array_type), dimensions),
            ),
            (2, Instruction::Return),
        ];
        let method = method(instructions, "()V", vec![]);
        assert_matches!(
            build(&method),
            Err(error) if error.kind == InvalidMultiArrayDimensions && error.pc == Some(1.into())
        );
    }
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
        Err(error) if error.kind == MissingInstruction && error.pc == Some(10.into())
    );
}

#[test]
fn reports_frame_sources_at_the_executed_instruction() {
    let method = method([(3, Instruction::IReturn)], "()I", vec![]);
    let error = build_failure(&method);
    assert_matches!(error, (Some(pc), StackUnderflow) if pc == 3.into());
}

#[test]
fn reports_incompatible_frame_shape_at_the_merge_target() {
    let instructions = [
        (0, Instruction::ILoad0),
        (1, Instruction::IfEq(6.into())),
        (4, Instruction::IConst0),
        (5, Instruction::Goto(6.into())),
        (6, Instruction::Return),
    ];
    let method = method(instructions, "(I)V", vec![]);

    let error = build(&method).expect_err("incoming stack shapes differ");
    assert_eq!(error.pc, Some(6.into()));
    assert_eq!(error.kind, IncompatibleFrameShape);
}

#[test]
fn reports_a_missing_fallthrough_at_the_source_instruction() {
    let fallthrough = method([(4, Instruction::Nop)], "()V", vec![]);
    assert_matches!(build(&fallthrough), Err(error) if error.kind == MissingFallthrough && error.pc == Some(4.into()));

    // Structural validation runs off the reachable path too.
    let unreachable = method(
        [(0, Instruction::Return), (1, Instruction::Nop)],
        "()V",
        vec![],
    );
    assert_matches!(build(&unreachable), Err(error) if error.kind == MissingFallthrough && error.pc == Some(1.into()));
}

#[test]
fn reports_method_entry_frame_initialization_failures() {
    let mut method = method([(0, Instruction::Return)], "(I)V", vec![]);
    method.body.as_mut().expect("method has a body").max_locals = 0;
    let error = build_failure(&method);
    assert_matches!(error, (None, LocalIndexOutOfBounds));
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
        assert_eq!(
            build_failure(&method),
            (Some(1.into()), UnsupportedLegacySubroutine)
        );
    }
}
