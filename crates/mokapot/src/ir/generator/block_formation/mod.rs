//! Forms semantic maximal blocks from analyzed JVM locations and frame facts.

mod layout;
mod materialize;
mod model;

#[cfg(test)]
mod tests;

pub(super) use model::JvmBlockGraph;
pub(in crate::ir::generator) use model::{JvmBlock, JvmBlockArm};

use crate::ir::generator::{error::MokaIRBuildError, jvm::analysis::AnalyzedJvmCfg};

use self::{
    layout::BlockLayout,
    materialize::{insert_entry_preheader, materialize_blocks},
};

/// Forms maximal semantic blocks from completed JVM frame facts.
pub(super) fn form(analyzed_cfg: AnalyzedJvmCfg) -> Result<JvmBlockGraph, MokaIRBuildError> {
    let layout = BlockLayout::discover(&analyzed_cfg)?;
    let phi_blocks = layout.phi_blocks(&analyzed_cfg.phi_values)?;
    let AnalyzedJvmCfg {
        initial_frame,
        locations,
        phi_values,
        this_value,
        parameter_values,
        ..
    } = analyzed_cfg;
    let blocks = materialize_blocks(locations, &layout)?;
    let blocks = insert_entry_preheader(blocks, &layout, initial_frame);

    Ok(JvmBlockGraph {
        entry: layout.entry(),
        blocks,
        phi_blocks,
        merge_values: phi_values,
        this_value,
        parameter_values,
    })
}
