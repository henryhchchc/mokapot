//! Materializes a semantic block layout into JVM blocks.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::JvmStackFrame,
                instruction::RegisterInstruction,
                normalization::Location,
                symbolic_execution::{AnalyzedLocation, JvmOutgoing, OperandState},
            },
        },
    },
    jvm::code::ProgramCounter,
};

use super::{
    layout::BlockLayout,
    model::{JvmBlock, JvmBlockArm},
};

pub(super) fn materialize_blocks(
    mut analyzed_locations: BTreeMap<Location, AnalyzedLocation>,
    layout: &BlockLayout,
) -> Result<Vec<JvmBlock>, MokaIRBuildError> {
    let blocks = layout
        .locations()
        .iter()
        .map(|(&id, locations)| materialize_block(id, locations, &mut analyzed_locations, layout))
        .collect::<Result<Vec<_>, _>>()?;
    if analyzed_locations.is_empty() {
        Ok(blocks)
    } else {
        Err(MokaIRBuildError::MalformedControlFlow)
    }
}

fn materialize_block(
    id: BlockId,
    locations: &[Location],
    analyzed_locations: &mut BTreeMap<Location, AnalyzedLocation>,
    layout: &BlockLayout,
) -> Result<JvmBlock, MokaIRBuildError> {
    let mut entry_frame = None;
    let mut caught_exception = None;
    let mut operations = Vec::with_capacity(locations.len());
    let mut terminator = None;
    let mut terminator_source = None;
    let mut arms = Vec::new();

    for (index, location) in locations.iter().copied().enumerate() {
        let AnalyzedLocation {
            incoming,
            instruction,
            outgoing,
            caught_exception: location_exception,
        } = analyzed_locations
            .remove(&location)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        if index == 0 {
            entry_frame = Some(incoming);
            caught_exception = location_exception;
        }
        let is_last = index + 1 == locations.len();
        if is_last {
            let end = materialize_block_end(location, instruction, outgoing, layout)?;
            operations.extend(end.operation);
            terminator = Some(end.terminator);
            terminator_source = end.terminator_source;
            arms = end.arms;
        } else {
            let next = locations[index + 1];
            let operation = materialize_internal_operation(location, instruction, &outgoing, next)?;
            operations.extend(operation);
        }
    }

    Ok(JvmBlock {
        id,
        entry_frame: entry_frame.ok_or(MokaIRBuildError::MalformedControlFlow)?,
        operations,
        terminator: terminator.ok_or(MokaIRBuildError::MalformedControlFlow)?,
        terminator_source,
        arms,
        caught_exception,
    })
}

fn materialize_internal_operation(
    location: Location,
    instruction: RegisterInstruction,
    outgoing: &[JvmOutgoing],
    next: Location,
) -> Result<Option<(ProgramCounter, OperationKind<OperandState>)>, MokaIRBuildError> {
    if instruction.is_explicit_transfer()
        || outgoing.len() != 1
        || outgoing[0].target != next
        || !matches!(outgoing[0].transfer, ControlTransfer::Unconditional)
    {
        return Err(MokaIRBuildError::MalformedControlFlow);
    }
    let operation = match instruction {
        RegisterInstruction::Definition { value, expr } => Some(OperationKind::Definition {
            value: OperandState::Value(value),
            expr,
        }),
        RegisterInstruction::Effect(expr) => Some(OperationKind::Effect { expr }),
        RegisterInstruction::Erased => None,
        RegisterInstruction::HandlerEntry
        | RegisterInstruction::Unwind
        | RegisterInstruction::Jump { .. }
        | RegisterInstruction::Switch { .. }
        | RegisterInstruction::Return(_)
        | RegisterInstruction::Throw(_)
        | RegisterInstruction::Subroutine { .. }
        | RegisterInstruction::SubroutineReturn(_) => {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
    };
    operation
        .map(|operation| {
            location
                .source_pc()
                .map(|pc| (pc, operation))
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .transpose()
}

struct BlockEnd {
    operation: Option<(ProgramCounter, OperationKind<OperandState>)>,
    terminator: TerminatorKind<OperandState>,
    terminator_source: Option<ProgramCounter>,
    arms: Vec<JvmBlockArm>,
}

fn materialize_block_end(
    location: Location,
    instruction: RegisterInstruction,
    outgoing: Vec<JvmOutgoing>,
    layout: &BlockLayout,
) -> Result<BlockEnd, MokaIRBuildError> {
    let explicit_transfer = instruction.is_explicit_transfer();
    let arms = outgoing
        .into_iter()
        .map(|outgoing| {
            layout
                .block_at(outgoing.target)
                .map(|target| JvmBlockArm {
                    target,
                    transfer: outgoing.transfer,
                    frame: outgoing.frame,
                })
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let has_normal_successor = arms
        .iter()
        .any(|arm| matches!(arm.transfer, ControlTransfer::Normal));
    let (operation, terminator) = classify_block_end(instruction, has_normal_successor);
    let operation = operation
        .map(|operation| {
            location
                .source_pc()
                .map(|pc| (pc, operation))
                .ok_or(MokaIRBuildError::MalformedControlFlow)
        })
        .transpose()?;

    Ok(BlockEnd {
        operation,
        terminator,
        terminator_source: explicit_transfer.then(|| location.source_pc()).flatten(),
        arms,
    })
}

pub(super) fn insert_entry_preheader(
    mut blocks: Vec<JvmBlock>,
    layout: &BlockLayout,
    initial_frame: JvmStackFrame<OperandState>,
) -> Vec<JvmBlock> {
    if layout.has_entry_preheader() {
        let entry_block = JvmBlock {
            id: layout.entry(),
            entry_frame: initial_frame.clone(),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            arms: vec![JvmBlockArm {
                target: layout.bytecode_entry(),
                transfer: ControlTransfer::Unconditional,
                frame: initial_frame,
            }],
            caught_exception: None,
        };
        blocks.insert(0, entry_block);
    }
    blocks
}

pub(super) fn classify_block_end(
    instruction: RegisterInstruction,
    has_normal_successor: bool,
) -> (
    Option<OperationKind<OperandState>>,
    TerminatorKind<OperandState>,
) {
    match instruction {
        RegisterInstruction::Unwind => (None, TerminatorKind::Unwind),
        RegisterInstruction::Jump {
            condition: Some(_), ..
        } => (None, TerminatorKind::Branch),
        RegisterInstruction::HandlerEntry
        | RegisterInstruction::Jump {
            condition: None, ..
        }
        | RegisterInstruction::Subroutine { .. }
        | RegisterInstruction::SubroutineReturn(_)
        | RegisterInstruction::Erased => (None, TerminatorKind::Goto),
        RegisterInstruction::Switch { match_value, .. } => {
            (None, TerminatorKind::Switch { match_value })
        }
        RegisterInstruction::Return(value) => (None, TerminatorKind::Return(value)),
        RegisterInstruction::Throw(value) => (None, TerminatorKind::Throw(value)),
        RegisterInstruction::Definition { value, expr } => (
            Some(OperationKind::Definition {
                value: OperandState::Value(value),
                expr,
            }),
            implicit_terminator(has_normal_successor),
        ),
        RegisterInstruction::Effect(expr) => (
            Some(OperationKind::Effect { expr }),
            implicit_terminator(has_normal_successor),
        ),
    }
}

const fn implicit_terminator(has_normal_successor: bool) -> TerminatorKind<OperandState> {
    if has_normal_successor {
        TerminatorKind::Fallible
    } else {
        TerminatorKind::Goto
    }
}
