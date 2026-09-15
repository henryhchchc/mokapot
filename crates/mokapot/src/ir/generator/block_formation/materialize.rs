//! Materializes a semantic block layout into JVM blocks.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            bytecode_analysis::{self, NodeAddress, RegisterInstruction, jvm::Frame},
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
};

use super::{
    layout::BlockLayout,
    model::{Arm, Block},
};

pub(super) fn materialize_blocks(
    mut instruction_nodes: BTreeMap<NodeAddress, bytecode_analysis::Node>,
    layout: &BlockLayout,
) -> Result<Vec<Block>, Error> {
    let blocks = layout
        .addrs()
        .iter()
        .map(|(&id, addrs)| materialize_block(id, addrs, &mut instruction_nodes, layout))
        .collect::<Result<Vec<_>, _>>()?;
    if instruction_nodes.is_empty() {
        Ok(blocks)
    } else {
        Err(Error::MalformedControlFlow)
    }
}

fn materialize_block(
    id: BlockId,
    addrs: &[NodeAddress],
    instruction_nodes: &mut BTreeMap<NodeAddress, bytecode_analysis::Node>,
    layout: &BlockLayout,
) -> Result<Block, Error> {
    let mut entry_frame = None;
    let mut caught_exception = None;
    let mut operations = Vec::with_capacity(addrs.len());
    let mut terminator = None;
    let mut terminator_source = None;
    let mut arms = Vec::new();

    for (index, addr) in addrs.iter().copied().enumerate() {
        let bytecode_analysis::Node {
            incoming_frame,
            instruction,
            can_throw_synchronously,
            outgoing_edges,
            caught_exception_value: location_exception,
        } = instruction_nodes
            .remove(&addr)
            .ok_or(Error::MalformedControlFlow)?;
        if index == 0 {
            entry_frame = Some(incoming_frame);
            caught_exception = location_exception;
        }
        let is_last = index + 1 == addrs.len();
        if is_last {
            let end = materialize_block_end(
                addr,
                instruction,
                can_throw_synchronously,
                outgoing_edges,
                layout,
            )?;
            operations.extend(end.operation);
            terminator = Some(end.terminator);
            terminator_source = end.terminator_source;
            arms = end.arms;
        } else {
            let next = addrs[index + 1];
            let operation = materialize_internal_operation(
                addr,
                instruction,
                can_throw_synchronously,
                &outgoing_edges,
                next,
            )?;
            operations.extend(operation);
        }
    }

    Ok(Block {
        id,
        entry_frame: entry_frame.ok_or(Error::MalformedControlFlow)?,
        operations,
        terminator: terminator.ok_or(Error::MalformedControlFlow)?,
        terminator_source,
        arms,
        caught_exception,
    })
}

fn materialize_internal_operation(
    addr: NodeAddress,
    instruction: RegisterInstruction,
    can_throw_synchronously: bool,
    outgoing: &[bytecode_analysis::Edge],
    next: NodeAddress,
) -> Result<Option<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>, Error> {
    if instruction.is_explicit_transfer()
        || can_throw_synchronously
        || outgoing.len() != 1
        || outgoing[0].target != next
    {
        return Err(Error::MalformedControlFlow);
    }
    let operation = match instruction {
        RegisterInstruction::Definition { value, expr } => Some(OperationKind::Definition {
            value: bytecode_analysis::Value::Ssa(value),
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
            return Err(Error::MalformedControlFlow);
        }
    };
    operation
        .map(|operation| {
            addr.source_pc()
                .map(|pc| (pc, operation))
                .ok_or(Error::MalformedControlFlow)
        })
        .transpose()
}

struct BlockEnd {
    operation: Option<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>,
    terminator: TerminatorKind<bytecode_analysis::Value>,
    terminator_source: Option<ProgramCounter>,
    arms: Vec<Arm>,
}

fn materialize_block_end(
    addr: NodeAddress,
    instruction: RegisterInstruction,
    can_throw_synchronously: bool,
    outgoing: Vec<bytecode_analysis::Edge>,
    layout: &BlockLayout,
) -> Result<BlockEnd, Error> {
    let explicit_transfer = instruction.is_explicit_transfer();
    let arms = outgoing
        .into_iter()
        .map(|outgoing| {
            layout
                .block_at(outgoing.target)
                .map(|target| Arm {
                    target,
                    transfer: outgoing.transfer,
                    frame: outgoing.target_frame,
                })
                .ok_or(Error::MalformedControlFlow)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (operation, terminator) = classify_block_end(instruction, can_throw_synchronously);
    let operation = operation
        .map(|operation| {
            addr.source_pc()
                .map(|pc| (pc, operation))
                .ok_or(Error::MalformedControlFlow)
        })
        .transpose()?;

    Ok(BlockEnd {
        operation,
        terminator,
        terminator_source: explicit_transfer.then(|| addr.source_pc()).flatten(),
        arms,
    })
}

pub(super) fn insert_entry_preheader(
    mut blocks: Vec<Block>,
    layout: &BlockLayout,
    initial_frame: Frame<bytecode_analysis::Value>,
) -> Vec<Block> {
    if layout.has_entry_preheader() {
        let entry_block = Block {
            id: layout.entry(),
            entry_frame: initial_frame.clone(),
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            arms: vec![Arm {
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

/// Converts a block-final instruction into its operation and terminator.
///
/// `is_fallible` reports whether the instruction can raise, which gives a
/// value- or effect-producing operation a [`TerminatorKind::Fallible`] end.
pub(super) fn classify_block_end(
    instruction: RegisterInstruction,
    is_fallible: bool,
) -> (
    Option<OperationKind<bytecode_analysis::Value>>,
    TerminatorKind<bytecode_analysis::Value>,
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
                value: bytecode_analysis::Value::Ssa(value),
                expr,
            }),
            implicit_terminator(is_fallible),
        ),
        RegisterInstruction::Effect(expr) => (
            Some(OperationKind::Effect { expr }),
            implicit_terminator(is_fallible),
        ),
    }
}

const fn implicit_terminator(is_fallible: bool) -> TerminatorKind<bytecode_analysis::Value> {
    if is_fallible {
        TerminatorKind::Fallible
    } else {
        TerminatorKind::Goto
    }
}
