//! Forms semantic maximal blocks from symbolic JVM nodes and frame facts.

mod layout;
mod materialize;
mod model;

#[cfg(test)]
mod tests;

pub(super) use model::Graph;
pub(crate) use model::{Arm, Block};

use crate::ir::generator::{error::MokaIRBuildError, jvm::symbolic_execution};

use self::{
    layout::BlockLayout,
    materialize::{insert_entry_preheader, materialize_blocks},
};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(symbolic_cfg: symbolic_execution::Cfg) -> Result<Graph, MokaIRBuildError> {
    let layout = BlockLayout::discover(&symbolic_cfg)?;
    let phi_blocks = layout.phi_blocks(&symbolic_cfg.phi_values)?;
    let symbolic_execution::Cfg {
        initial_frame,
        nodes,
        phi_values,
        receiver_value,
        parameter_values,
        ..
    } = symbolic_cfg;
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
