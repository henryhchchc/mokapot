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

use self::{
    layout::BlockLayout,
    materialize::{insert_entry_preheader, materialize_blocks},
};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(node_graph: NodeGraph) -> Result<BlockGraph, Error> {
    let layout = BlockLayout::discover(&node_graph)?;
    let phi_blocks = layout.phi_blocks(&node_graph.phi_values)?;
    let NodeGraph {
        initial_frame,
        nodes,
        phi_values,
        receiver_value,
        parameter_values,
        ..
    } = node_graph;
    let blocks = materialize_blocks(nodes, &layout)?;
    let blocks = insert_entry_preheader(blocks, &layout, initial_frame);

    Ok(BlockGraph {
        entry: layout.entry(),
        blocks,
        phi_blocks,
        merge_values: phi_values,
        this_value: receiver_value,
        parameter_values,
    })
}

/// Block-level JVM graph consumed by SSA construction.
pub(super) struct BlockGraph {
    pub entry: BlockId,
    pub blocks: Vec<Block>,
    pub phi_blocks: BTreeMap<SsaValueId, BlockId>,
    pub merge_values: BTreeMap<FrameMergeSite, SsaValueId>,
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
