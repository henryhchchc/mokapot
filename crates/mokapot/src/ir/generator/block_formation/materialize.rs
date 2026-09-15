//! Materializes a semantic block layout into JVM blocks.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            bytecode_analysis::{self, Node, NodeAddress, RegisterInstruction, jvm::Frame},
            error::Error,
            identity::SsaValueId,
        },
    },
    jvm::code::ProgramCounter,
};

use super::{
    Arm, Block, BlockEnd,
    layout::{BlockLayout, LayoutBlock},
};

/// Materializes the blocks of `layout` in ascending block id order.
///
/// The layout owns its nodes, so each is materialized exactly once.
pub(super) fn materialize_blocks(layout: BlockLayout) -> Result<Vec<Block>, Error> {
    let BlockLayout {
        blocks,
        block_by_addr,
        ..
    } = layout;
    blocks
        .into_iter()
        .enumerate()
        .map(|(index, block)| {
            let id = BlockId::new(
                u32::try_from(index)
                    .map_err(|_| Error::internal("the block identity space is exhausted"))?,
            );
            match block {
                LayoutBlock::EntryPreheader { frame, target } => entry_preheader(id, frame, target),
                LayoutBlock::Nodes(nodes) => materialize_block(id, nodes, &block_by_addr),
            }
        })
        .collect()
}

/// Materializes the synthetic entry preheader.
///
/// The preheader has no operation of its own: it enters the method frame and
/// hands it to the bytecode entry block.
fn entry_preheader(
    id: BlockId,
    frame: Frame<bytecode_analysis::Value>,
    target: BlockId,
) -> Result<Block, Error> {
    Ok(Block {
        id,
        entry_frame: frame.clone(),
        operations: Vec::new(),
        end: BlockEnd::new(
            TerminatorKind::Goto,
            None,
            vec![Arm {
                target,
                transfer: ControlTransfer::Unconditional,
                frame,
            }],
        )?,
        caught_exception: None,
    })
}

fn materialize_block(
    id: BlockId,
    nodes: Vec<(NodeAddress, Node)>,
    block_by_addr: &BTreeMap<NodeAddress, BlockId>,
) -> Result<Block, Error> {
    let mut entry_frame = None;
    let mut caught_exception = None;
    let mut operations = Vec::with_capacity(nodes.len());
    let mut end = None;

    let mut nodes = nodes.into_iter().enumerate().peekable();
    while let Some((index, (addr, node))) = nodes.next() {
        let next = nodes.peek().map(|(_, (next, _))| *next);
        let has_exceptional_exit = node.has_exceptional_exit();
        if let Some(next) = next {
            // Layout cut a block here only if a node does not elide into its
            // successor, so only the last node of a block may fail this.
            debug_assert!(
                node.elides_into(next),
                "a block interior elides into the node after it"
            );
        }
        let Node {
            incoming_frame,
            instruction,
            outgoing_edges,
        } = node;
        if index == 0 {
            caught_exception = caught_exception_at(addr, &incoming_frame)?;
            entry_frame = Some(incoming_frame);
        }
        if next.is_none() {
            let MaterializedEnd {
                operation,
                end: block_end,
            } = materialize_block_end(
                addr,
                instruction,
                has_exceptional_exit,
                outgoing_edges,
                block_by_addr,
            )?;
            if let Some(operation) = operation {
                operations.push(operation);
            }
            end = Some(block_end);
        } else if let Some(operation) = materialize_internal_operation(addr, instruction)? {
            operations.push(operation);
        }
    }

    Ok(Block {
        id,
        entry_frame: entry_frame
            .ok_or_else(|| Error::internal("a formed block has no entry frame"))?,
        operations,
        end: end.ok_or_else(|| Error::internal("a formed block has no terminator"))?,
        caught_exception,
    })
}

fn caught_exception_at(
    addr: NodeAddress,
    incoming_frame: &Frame<bytecode_analysis::Value>,
) -> Result<Option<SsaValueId>, Error> {
    if !addr.is_handler() {
        return Ok(None);
    }
    let exception = incoming_frame
        .handler_exception()
        .map_err(|_| Error::internal("a handler entry frame lacks exactly one exception"))?;
    match exception {
        bytecode_analysis::Value::Ssa(value) => Ok(Some(*value)),
        bytecode_analysis::Value::ReturnAddress(_)
        | bytecode_analysis::Value::Merged(_)
        | bytecode_analysis::Value::Invalid => Err(Error::internal(
            "a handler entry frame has no synthesized exception identity",
        )),
    }
}

/// Converts a node that is elided into the next node of its block.
///
/// Layout elides only an ordinary, non-fallible operation, so its end is a
/// plain fallthrough.
fn materialize_internal_operation(
    addr: NodeAddress,
    instruction: RegisterInstruction,
) -> Result<Option<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>, Error> {
    let (operation, _) = classify_block_end(instruction, false);
    operation
        .map(|operation| {
            addr.source_pc()
                .map(|pc| (pc, operation))
                .ok_or_else(|| Error::internal("an operation has no source instruction"))
        })
        .transpose()
}

/// The semantic contents contributed by a block-final instruction.
struct MaterializedEnd {
    operation: Option<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>,
    end: BlockEnd,
}

fn materialize_block_end(
    addr: NodeAddress,
    instruction: RegisterInstruction,
    has_exceptional_exit: bool,
    outgoing: Vec<bytecode_analysis::Edge>,
    block_by_addr: &BTreeMap<NodeAddress, BlockId>,
) -> Result<MaterializedEnd, Error> {
    let explicit_transfer = instruction.is_explicit_transfer();
    let arms = outgoing
        .into_iter()
        .map(|outgoing| {
            block_by_addr
                .get(&outgoing.target)
                .map(|&target| Arm {
                    target,
                    transfer: outgoing.transfer,
                    frame: outgoing.target_frame,
                })
                .ok_or_else(|| Error::internal("a control-flow edge targets no formed block"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    let (operation, terminator) = classify_block_end(instruction, has_exceptional_exit);
    let operation = operation
        .map(|operation| {
            addr.source_pc()
                .map(|pc| (pc, operation))
                .ok_or_else(|| Error::internal("an operation has no source instruction"))
        })
        .transpose()?;

    Ok(MaterializedEnd {
        operation,
        end: BlockEnd::new(
            terminator,
            explicit_transfer.then(|| addr.source_pc()).flatten(),
            arms,
        )?,
    })
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
