//! Materializes a semantic block layout into JVM blocks.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            error::Error,
            instruction_graph::{self, NodeAddress, RegisterInstruction, frame::Frame},
        },
    },
    jvm::code::ProgramCounter,
};

use super::{
    layout::BlockLayout,
    model::{Arm, Block},
};

pub(super) fn materialize_blocks(
    mut instruction_nodes: BTreeMap<NodeAddress, instruction_graph::Node>,
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
    instruction_nodes: &mut BTreeMap<NodeAddress, instruction_graph::Node>,
    layout: &BlockLayout,
) -> Result<Block, Error> {
    let mut entry_frame = None;
    let mut caught_exception = None;
    let mut operations = Vec::with_capacity(addrs.len());
    let mut terminator = None;
    let mut terminator_source = None;
    let mut arms = Vec::new();

    for (index, addr) in addrs.iter().copied().enumerate() {
        let instruction_graph::Node {
            incoming_frame,
            instruction,
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
            let end = materialize_block_end(addr, instruction, outgoing_edges, layout)?;
            operations.extend(end.operation);
            terminator = Some(end.terminator);
            terminator_source = end.terminator_source;
            arms = end.arms;
        } else {
            let next = addrs[index + 1];
            let operation =
                materialize_internal_operation(addr, instruction, &outgoing_edges, next)?;
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
    outgoing: &[instruction_graph::Edge],
    next: NodeAddress,
) -> Result<Option<(ProgramCounter, OperationKind<instruction_graph::Value>)>, Error> {
    if instruction.is_explicit_transfer()
        || outgoing.len() != 1
        || outgoing[0].target != next
        || !matches!(outgoing[0].transfer, ControlTransfer::Unconditional)
    {
        return Err(Error::MalformedControlFlow);
    }
    let operation = match instruction {
        RegisterInstruction::Definition { value, expr } => Some(OperationKind::Definition {
            value: instruction_graph::Value::Ssa(value),
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
    operation: Option<(ProgramCounter, OperationKind<instruction_graph::Value>)>,
    terminator: TerminatorKind<instruction_graph::Value>,
    terminator_source: Option<ProgramCounter>,
    arms: Vec<Arm>,
}

fn materialize_block_end(
    addr: NodeAddress,
    instruction: RegisterInstruction,
    outgoing: Vec<instruction_graph::Edge>,
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
    let has_normal_successor = arms
        .iter()
        .any(|arm| matches!(arm.transfer, ControlTransfer::Normal));
    let (operation, terminator) = classify_block_end(instruction, has_normal_successor);
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
    initial_frame: Frame<instruction_graph::Value>,
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

pub(super) fn classify_block_end(
    instruction: RegisterInstruction,
    has_normal_successor: bool,
) -> (
    Option<OperationKind<instruction_graph::Value>>,
    TerminatorKind<instruction_graph::Value>,
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
                value: instruction_graph::Value::Ssa(value),
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

const fn implicit_terminator(
    has_normal_successor: bool,
) -> TerminatorKind<instruction_graph::Value> {
    if has_normal_successor {
        TerminatorKind::Fallible
    } else {
        TerminatorKind::Goto
    }
}
