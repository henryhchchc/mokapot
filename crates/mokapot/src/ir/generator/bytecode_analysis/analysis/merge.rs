//! Entry-frame merging and complete block-parameter maintenance.

use std::collections::BTreeMap;

use super::super::values::ValueContext;
use super::{Analyzer, Contribution, Frame, ParameterDefinition, ParameterSite};
use crate::ir::{BlockId, ValueId, generator::error::Error};

impl Analyzer<'_, '_> {
    pub(super) fn recompute_entry(&mut self, block: BlockId) -> Result<bool, Error> {
        let block_pc = self.block_pc(block);
        let contributions = self
            .blocks
            .get(&block)
            .ok_or_else(|| Error::internal("a reachable block has no analysis state"))?
            .contributions
            .iter()
            .map(|(&predecessor, frame)| (predecessor, frame.clone()))
            .collect::<Vec<_>>();
        let mut frames = contributions.iter().map(|(_, frame)| frame.clone());
        let mut merged = frames
            .next()
            .ok_or_else(|| Error::internal("a reachable block has no predecessor frame"))?;
        let mut active_parameters = BTreeMap::new();

        for contribution in frames {
            let existing_parameters = &self.parameter_definitions;
            let values = &mut self.values;
            merged
                .merge_from_with(contribution, |position, lhs, rhs| {
                    merge_value(
                        ParameterSite { block, position },
                        lhs,
                        rhs,
                        existing_parameters,
                        &mut active_parameters,
                        values,
                    )
                })
                .map_err(|error| error.at_instruction(block_pc))?;
        }

        let parameter_definitions =
            synchronize_parameter_definitions(&merged, &contributions, active_parameters)
                .map_err(|error| error.at_instruction(block_pc))?;

        self.parameter_definitions
            .retain(|site, _| site.block != block);
        self.parameter_definitions.extend(parameter_definitions);

        let execution = &mut self
            .blocks
            .get_mut(&block)
            .ok_or_else(|| Error::internal("a reachable block has no analysis state"))?
            .execution;
        Ok(execution.update_input(merged))
    }
}

fn merge_value(
    site: ParameterSite,
    lhs: &mut ValueId,
    rhs: ValueId,
    existing_parameters: &BTreeMap<ParameterSite, ParameterDefinition>,
    active_parameters: &mut BTreeMap<ParameterSite, ValueId>,
    values: &mut ValueContext,
) -> Result<(), Error> {
    if *lhs == rhs {
        return Ok(());
    }
    let result = if let Some(&result) = active_parameters.get(&site) {
        result
    } else {
        let result = existing_parameters
            .get(&site)
            .map_or_else(|| values.fresh(), |definition| Ok(definition.result))?;
        active_parameters.insert(site, result);
        result
    };

    *lhs = result;
    Ok(())
}

fn edge_arguments(
    site: ParameterSite,
    contributions: &[(Contribution, Frame)],
) -> Result<BTreeMap<Contribution, ValueId>, Error> {
    contributions
        .iter()
        .map(|(contribution, frame)| {
            let value = frame
                .value_at(site.position)
                .copied()
                .ok_or_else(|| Error::internal("an edge argument frame lacks its merged slot"))?;
            Ok((*contribution, value))
        })
        .collect()
}

fn synchronize_parameter_definitions(
    merged: &Frame,
    contributions: &[(Contribution, Frame)],
    active_parameters: BTreeMap<ParameterSite, ValueId>,
) -> Result<BTreeMap<ParameterSite, ParameterDefinition>, Error> {
    active_parameters
        .into_iter()
        .filter_map(|(site, result)| {
            let merged_value = merged.value_at(site.position).copied();
            (merged_value == Some(result)).then_some((site, result))
        })
        .map(|(site, result)| {
            edge_arguments(site, contributions).map(|inputs| {
                let definition = ParameterDefinition { result, inputs };
                (site, definition)
            })
        })
        .collect()
}
