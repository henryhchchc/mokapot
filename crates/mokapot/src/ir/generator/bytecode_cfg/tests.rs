use std::collections::BTreeMap;

use super::*;
use crate::{
    ir::generator::{
        error::{Error, MalformedBytecode},
        tests::method,
    },
    jvm::code::{ExceptionTableEntry, Instruction},
};

#[test]
fn partitions_unreachable_bytecode_after_control_transfers() {
    let instructions = [
        (0, Instruction::Nop),
        (1, Instruction::Goto(4.into())),
        (2, Instruction::Nop),
        (3, Instruction::Return),
        (4, Instruction::Return),
    ];
    let method = method(instructions, "()V", vec![]);

    let cfg = build(&method).unwrap();
    let blocks = cfg.blocks().collect::<Vec<_>>();

    let insn_pcs = blocks
        .iter()
        .map(|block| block.instruction_pcs.as_slice())
        .collect::<Vec<_>>();
    let expected = vec![
        vec![0.into(), 1.into()],
        vec![2.into(), 3.into()],
        vec![4.into()],
    ];
    assert_eq!(insn_pcs, expected);
    assert_eq!(cfg.entry_block(), StructuralBlockId::from_index(0));
    assert_eq!(
        blocks[0].terminator,
        StructuralTerminator::Goto {
            target: StructuralBlockId::from_index(2)
        }
    );
}

#[test]
fn keeps_fallible_boundaries_and_synthetic_exception_targets() {
    let string_type = "java/lang/String".parse().unwrap();
    let instructions = [
        (0, Instruction::AConstNull),
        (1, Instruction::CheckCast(string_type)),
        (2, Instruction::Return),
        (10, Instruction::AStore0),
        (11, Instruction::Return),
    ];
    let exception_table = vec![ExceptionTableEntry {
        covered_pc: 1.into()..2.into(),
        handler_pc: 10.into(),
        catch_type: Some("java/lang/RuntimeException".parse().unwrap()),
    }];
    let method = method(instructions, "()V", exception_table);

    let cfg = build(&method).unwrap();
    let blocks = cfg.blocks().collect::<Vec<_>>();
    let fallible = blocks
        .iter()
        .find(|block| block.instruction_pcs.contains(&1.into()))
        .unwrap();
    let handler_id = HandlerId::from_index(0);
    let handler = cfg.handler(handler_id).unwrap();

    assert_eq!(fallible.instruction_pcs, [0.into(), 1.into()]);
    assert_eq!(
        fallible.terminator,
        StructuralTerminator::Fallthrough {
            target: cfg.block_id_at_pc(2.into()).unwrap(),
        }
    );
    assert_eq!(
        fallible.exceptional_successors,
        vec![
            ExceptionalTarget::Handler(handler_id),
            ExceptionalTarget::Unwind
        ]
    );
    assert_eq!(handler.target, cfg.block_id_at_pc(10.into()).unwrap());
}

#[test]
fn validates_targets_even_in_unreachable_bytecode() {
    let method = method(
        [(0, Instruction::Return), (1, Instruction::Goto(10.into()))],
        "()V",
        vec![],
    );

    let error = build(&method).unwrap_err();

    assert!(matches!(
        error,
        Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::MissingInstruction,
        } if pc == 10.into()
    ));
}

#[test]
fn validates_required_fallthroughs_in_unreachable_bytecode() {
    let method = method(
        [(0, Instruction::Return), (1, Instruction::Nop)],
        "()V",
        vec![],
    );

    let error = build(&method).unwrap_err();

    assert!(matches!(
        error,
        Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::MissingFallthrough,
        } if pc == 1.into()
    ));
}

#[test]
fn rejects_mismatched_table_switch_cardinality() {
    let instruction = Instruction::TableSwitch {
        range: 1..=3,
        jump_targets: vec![10.into(), 10.into()],
        default: 10.into(),
    };
    let method = method([(0, instruction), (10, Instruction::Return)], "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidTableSwitch,
        }) if pc == 0.into()
    ));
}

#[test]
fn rejects_reversed_table_switch_ranges_even_without_targets() {
    let switch = Instruction::TableSwitch {
        range: std::ops::RangeInclusive::new(3, 1),
        jump_targets: vec![],
        default: 10.into(),
    };
    let method = method([(0, switch), (10, Instruction::Return)], "()V", vec![]);

    assert!(matches!(
        build(&method),
        Err(Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidTableSwitch,
        }) if pc == 0.into()
    ));
}

#[test]
fn rejects_reversed_exception_ranges() {
    let exception_table = vec![ExceptionTableEntry {
        covered_pc: 10.into()..0.into(),
        handler_pc: 10.into(),
        catch_type: None,
    }];
    let instructions = [(0, Instruction::Return), (10, Instruction::Return)];
    let method = method(instructions, "()V", exception_table);

    assert!(matches!(
        build(&method),
        Err(Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidExceptionRange,
        }) if pc == 10.into()
    ));
}

#[test]
fn rejects_exception_range_end_between_instruction_boundaries() {
    let instructions = [(0, Instruction::SiPush(0)), (3, Instruction::Return)];
    let exception_table = vec![ExceptionTableEntry {
        covered_pc: 0.into()..2.into(),
        handler_pc: 3.into(),
        catch_type: None,
    }];
    let method = method(instructions, "()V", exception_table);

    assert!(matches!(
        build(&method),
        Err(Error::MalformedBytecode {
            pc: Some(pc),
            kind: MalformedBytecode::InvalidExceptionRange,
        }) if pc == 2.into()
    ));
}

#[test]
fn accepts_exception_range_ending_after_the_last_instruction_start() {
    let instructions = [
        (0, Instruction::Nop),
        (3, Instruction::Return),
        (4, Instruction::Return),
    ];
    let exception_table = vec![ExceptionTableEntry {
        covered_pc: 0.into()..5.into(),
        handler_pc: 4.into(),
        catch_type: None,
    }];
    let method = method(instructions, "()V", exception_table);

    build(&method).unwrap();
}

#[test]
fn retains_switch_case_labels_and_parallel_targets() {
    let switch = Instruction::LookupSwitch {
        default: 3.into(),
        match_targets: BTreeMap::from([(1, 2.into()), (2, 2.into())]),
    };
    let instructions = [
        (0, switch),
        (1, Instruction::Return),
        (2, Instruction::Return),
        (3, Instruction::Return),
    ];
    let method = method(instructions, "()V", vec![]);

    let cfg = build(&method).unwrap();
    let switch = cfg.blocks().next().unwrap();
    let target_at = |pc: u16| cfg.block_id_at_pc(pc.into()).unwrap();

    assert_eq!(
        switch.terminator,
        StructuralTerminator::Switch {
            cases: BTreeMap::from([(1, target_at(2)), (2, target_at(2))]),
            default: target_at(3),
        }
    );
}

#[test]
fn structural_terminators_retain_branch_and_return_operand_shapes() {
    let instructions = [
        (0, Instruction::IConst0),
        (1, Instruction::IfEq(5.into())),
        (4, Instruction::Return),
        (5, Instruction::LReturn),
    ];
    let method = method(instructions, "()V", vec![]);

    let cfg = build(&method).unwrap();
    assert!(matches!(
        cfg.block_at_pc(0.into()).unwrap().terminator,
        StructuralTerminator::Branch {
            predicate: BranchPredicate::IsZero,
            ..
        }
    ));
    assert_eq!(
        cfg.block_at_pc(5.into()).unwrap().terminator,
        StructuralTerminator::Return {
            operand: ReturnOperand::Category2
        }
    );
}
