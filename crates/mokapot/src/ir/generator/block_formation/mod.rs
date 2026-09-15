//! Forms semantic maximal blocks from JVM instruction nodes and frame facts.

mod layout;
mod materialize;
mod model;

#[cfg(test)]
mod tests;

pub(super) use model::Graph;
pub(crate) use model::{Arm, Block};

use crate::ir::generator::{error::Error, instruction_graph};

use self::{
    layout::BlockLayout,
    materialize::{insert_entry_preheader, materialize_blocks},
};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(instruction_graph: instruction_graph::Graph) -> Result<Graph, Error> {
    let layout = BlockLayout::discover(&instruction_graph)?;
    let phi_blocks = layout.phi_blocks(&instruction_graph.phi_values)?;
    let instruction_graph::Graph {
        initial_frame,
        nodes,
        phi_values,
        receiver_value,
        parameter_values,
        ..
    } = instruction_graph;
    let blocks = materialize_blocks(nodes, &layout)?;
    let blocks = insert_entry_preheader(blocks, &layout, initial_frame);

    Ok(Graph {
        entry: layout.entry(),
        blocks,
        phi_blocks,
        merge_values: phi_values,
        this_value: receiver_value,
        parameter_values,
    })
}
