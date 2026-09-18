//! Conversion from analyzed frames to a frame-free scalar graph.

mod model;

pub(crate) use model::{PhiCandidate, ScalarBlock, ScalarGraph};

use std::collections::BTreeMap;

use itertools::Itertools;

use super::analysis::{
    CompletedAnalysis, LiftedBlock, Location, PhiDefinition, PhiSite, Predecessor,
};
use crate::ir::{
    BlockId, TerminatorKind, ValueId, control_flow::ControlTransfer, generator::error::Error,
};

pub(super) fn materialize(
    completed: CompletedAnalysis,
    entry_location: Location,
) -> Result<ScalarGraph, Error> {
    let CompletedAnalysis {
        locations,
        phi_definitions,
        receiver_value,
        parameter_values,
    } = completed;
    let has_preheader = locations.get(&entry_location).is_some_and(|state| {
        state
            .contributions
            .keys()
            .any(|predecessor| *predecessor != Predecessor::Entry)
    });
    let analyzed_locations = locations
        .iter()
        .filter_map(|(&location, state)| state.execution.block().map(|_| location))
        .collect::<Vec<_>>();
    let offset = usize::from(has_preheader);
    let block_ids_by_location = analyzed_locations
        .iter()
        .enumerate()
        .map(|(index, &location)| scalar_block_id(index + offset).map(|id| (location, id)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;
    let entry_block = *block_ids_by_location
        .get(&entry_location)
        .ok_or_else(|| Error::internal("the entry location has no scalar block"))?;
    let preheader = has_preheader.then(|| BlockId::new(0));
    let entry = preheader.unwrap_or(entry_block);

    let preheader_block = preheader.map(|id| {
        Ok(ScalarBlock {
            id,
            caught_exception: None,
            operations: Vec::new(),
            terminator: TerminatorKind::Goto,
            terminator_source: None,
            successors: vec![(entry_block, ControlTransfer::Unconditional)],
        })
    });
    let location_blocks = analyzed_locations.iter().map(|location| {
        let analyzed = locations
            .get(location)
            .and_then(|state| state.execution.block().cloned())
            .ok_or_else(|| Error::internal("a reachable location was not executed"))?;
        materialize_block(
            analyzed,
            *block_ids_by_location
                .get(location)
                .ok_or_else(|| Error::internal("an executed location has no scalar block"))?,
            &block_ids_by_location,
        )
    });
    let blocks = preheader_block
        .into_iter()
        .chain(location_blocks)
        .try_collect()?;

    let phi_candidates = materialize_phis(&phi_definitions, &block_ids_by_location, preheader)?;
    Ok(ScalarGraph {
        entry,
        blocks,
        phi_candidates,
        this_value: receiver_value,
        parameter_values,
    })
}

fn materialize_phis(
    phi_definitions: &BTreeMap<PhiSite, PhiDefinition>,
    block_ids_by_location: &BTreeMap<Location, BlockId>,
    preheader: Option<BlockId>,
) -> Result<BTreeMap<ValueId, PhiCandidate>, Error> {
    phi_definitions
        .iter()
        .filter(|(site, _)| block_ids_by_location.contains_key(&site.location))
        .map(|(site, definition)| {
            materialize_phi(site.location, definition, block_ids_by_location, preheader)
                .map(|candidate| (definition.result, candidate))
        })
        .collect()
}

fn materialize_phi(
    location: Location,
    definition: &PhiDefinition,
    block_ids_by_location: &BTreeMap<Location, BlockId>,
    preheader: Option<BlockId>,
) -> Result<PhiCandidate, Error> {
    let inputs = definition
        .inputs
        .iter()
        .map(|(predecessor, &input)| {
            let predecessor = match predecessor {
                Predecessor::Entry => preheader
                    .ok_or_else(|| Error::internal("an entry phi has no synthetic predecessor"))?,
                Predecessor::Location(location) => {
                    *block_ids_by_location.get(location).ok_or_else(|| {
                        Error::internal("a phi input predecessor has no scalar block")
                    })?
                }
            };
            Ok((predecessor, input))
        })
        .collect::<Result<_, Error>>()?;
    let placement = *block_ids_by_location
        .get(&location)
        .ok_or_else(|| Error::internal("a phi definition has no scalar block"))?;
    Ok(PhiCandidate { placement, inputs })
}

fn materialize_block(
    analyzed: LiftedBlock,
    id: BlockId,
    block_ids_by_location: &BTreeMap<Location, BlockId>,
) -> Result<ScalarBlock, Error> {
    let successors = analyzed
        .successors
        .edges
        .into_iter()
        .map(|successor| {
            let target = *block_ids_by_location
                .get(&successor.target)
                .ok_or_else(|| Error::internal("a successor target has no scalar block"))?;
            Ok((target, successor.transfer))
        })
        .collect::<Result<_, Error>>()?;
    let operations = analyzed.operations;
    let terminator = analyzed.terminator;
    Ok(ScalarBlock {
        id,
        caught_exception: analyzed.caught_exception,
        operations,
        terminator,
        terminator_source: analyzed.terminator_source,
        successors,
    })
}

fn scalar_block_id(index: usize) -> Result<BlockId, Error> {
    u32::try_from(index)
        .map(BlockId::new)
        .map_err(|_| Error::internal("the block identity space is exhausted"))
}
