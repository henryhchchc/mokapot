use super::*;
use crate::{
    ir::{MalformedBytecode, MokaIRFrameError, UnsupportedBytecode},
    jvm::code::WideInstruction,
};

#[test]
fn reports_empty_code_at_the_method_boundary() {
    let method = method(std::iter::empty::<(u16, Instruction)>(), "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: None,
            kind: MalformedBytecode::MissingEntry,
        })
    ));
}

#[test]
fn reports_a_missing_jump_target_at_that_target() {
    let method = method([(0, Instruction::Goto(10.into()))], "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::MissingInstruction,
        }) if pc == 10.into()
    ));
}

#[test]
fn rejects_a_missing_structural_target_even_when_its_source_is_unreachable() {
    let method = method(
        [(0, Instruction::Return), (1, Instruction::Goto(10.into()))],
        "()V",
        vec![],
    );

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::MissingInstruction,
        }) if pc == 10.into()
    ));
}

#[test]
fn reports_frame_sources_at_the_executed_instruction() {
    let method = method([(3, Instruction::IReturn)], "()I", vec![]);

    let error = build(&method).expect_err("the empty operand stack cannot return a value");
    assert!(matches!(
        error,
        MokaIRBuildError::InvalidFrame {
            pc: Some(pc),
            source: MokaIRFrameError::StackUnderflow,
        } if pc == 3.into()
    ));
}

#[test]
fn reports_a_missing_fallthrough_at_the_source_instruction() {
    let method = method([(4, Instruction::Nop)], "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::MissingFallthrough,
        }) if pc == 4.into()
    ));
}

#[test]
fn reports_method_entry_frame_initialization_failures() {
    let mut method = method([(0, Instruction::Return)], "(I)V", vec![]);
    method.body.as_mut().expect("method has a body").max_locals = 0;

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::InvalidFrame {
            pc: None,
            source: MokaIRFrameError::LocalIndexOutOfBounds,
        })
    ));
}

#[test]
fn rejects_every_legacy_subroutine_instruction_even_when_unreachable() {
    for instruction in [
        Instruction::Jsr(0.into()),
        Instruction::JsrW(0.into()),
        Instruction::Ret(0),
        Instruction::Wide(WideInstruction::Ret(300)),
    ] {
        let method = method([(0, Instruction::Return), (1, instruction)], "()V", vec![]);

        assert!(matches!(
            build(&method),
            Err(MokaIRBuildError::UnsupportedBytecode {
                pc,
                kind: UnsupportedBytecode::LegacySubroutine,
            }) if pc == 1.into()
        ));
    }
}

#[test]
fn rejects_an_inconsistent_table_switch() {
    let switch = Instruction::TableSwitch {
        range: 1..=3,
        jump_targets: vec![10.into(), 10.into()],
        default: 10.into(),
    };
    let method = method([(0, switch), (10, Instruction::Return)], "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidTableSwitch,
        }) if pc == 0.into()
    ));
}

#[test]
fn rejects_an_unaligned_exception_range() {
    let method = method(
        [(0, Instruction::SiPush(0)), (3, Instruction::Return)],
        "()V",
        vec![ExceptionTableEntry {
            covered_pc: 0.into()..2.into(),
            handler_pc: 3.into(),
            catch_type: None,
        }],
    );

    assert!(matches!(
        build(&method),
        Err(MokaIRBuildError::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidExceptionRange,
        }) if pc == 2.into()
    ));
}
