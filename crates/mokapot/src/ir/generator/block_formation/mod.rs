//! Forms semantic maximal blocks from JVM instruction nodes and frame facts.

mod layout;
mod materialize;

#[cfg(test)]
mod tests;

use crate::{
    ir::{
        BlockId, OperationKind, TerminatorKind,
        control_flow::ControlTransfer,
        generator::{
            bytecode_analysis::{self, FrameMergeSite, jvm::Frame},
            error::Error,
            identity::SsaValueId,
        },
    },
    jvm::code::ProgramCounter,
};
use std::collections::BTreeMap;

use super::bytecode_analysis::NodeGraph;

use self::{layout::BlockLayout, materialize::materialize_blocks};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(node_graph: NodeGraph) -> Result<BlockGraph, Error> {
    let NodeGraph {
        entry_addr,
        initial_frame,
        nodes,
        phi_values,
        receiver_value,
        parameter_values,
    } = node_graph;
    let layout = BlockLayout::discover(nodes, entry_addr, initial_frame, &phi_values)?;
    // A site, its value, and its block are one relation: joining the layout's
    // sites here is what forms it, and SSA consumes it alone.
    let merges = layout
        .phi_blocks
        .iter()
        .map(|(&site, &block)| {
            let value = phi_values[&site];
            (site, Merge { value, block })
        })
        .collect();
    let entry = layout.entry;
    let blocks = materialize_blocks(layout)?;
    debug_assert!(
        blocks
            .iter()
            .enumerate()
            .all(|(index, block)| usize::try_from(block.id.index()).is_ok_and(|id| id == index)),
        "a block id must index the blocks vector"
    );

    Ok(BlockGraph {
        entry,
        blocks,
        merges,
        this_value: receiver_value,
        parameter_values,
    })
}

/// A frame merge site resolved to the value that stands for it and the block
/// that computes it.
#[derive(Debug)]
pub(crate) struct Merge {
    pub value: SsaValueId,
    pub block: BlockId,
}

/// Block-level JVM graph consumed by SSA construction.
pub(super) struct BlockGraph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    /// The block and provisional value of every frame merge site.
    ///
    /// A site, its value, and the block that computes it are one relation, so
    /// it is carried once rather than recomposed.
    pub merges: BTreeMap<FrameMergeSite, Merge>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}

/// One exact outgoing edge from a formed JVM block.
#[derive(Debug)]
pub(crate) struct Arm {
    pub target: BlockId,
    pub transfer: ControlTransfer<bytecode_analysis::Value>,
    pub frame: Frame<bytecode_analysis::Value>,
}

/// A maximal JVM block with exact register operands and outgoing frames.
#[derive(Debug)]
pub(crate) struct Block {
    pub id: BlockId,
    pub entry_frame: Frame<bytecode_analysis::Value>,
    pub operations: Vec<(ProgramCounter, OperationKind<bytecode_analysis::Value>)>,
    pub terminator: TerminatorKind<bytecode_analysis::Value>,
    pub terminator_source: Option<ProgramCounter>,
    pub arms: Vec<Arm>,
    pub caught_exception: Option<SsaValueId>,
}
