//! Entry-frame merging and complete phi-definition maintenance.

use std::collections::BTreeMap;

use super::super::values::ValueContext;
use super::{Analyzer, Frame, PhiDefinition, PhiSite, Predecessor};
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
        let mut active_phis = BTreeMap::new();

        for contribution in frames {
            let existing_phis = &self.phi_definitions;
            let values = &mut self.values;
            merged
                .merge_from_with(contribution, |position, lhs, rhs| {
                    merge_value(
                        PhiSite { block, position },
                        lhs,
                        rhs,
                        existing_phis,
                        &mut active_phis,
                        values,
                    )
                })
                .map_err(|error| error.at_instruction_if_present(block_pc))?;
        }

        let phi_definitions = synchronize_phi_definitions(&merged, &contributions, active_phis)
            .map_err(|error| error.at_instruction_if_present(block_pc))?;

        self.phi_definitions.retain(|site, _| site.block != block);
        self.phi_definitions.extend(phi_definitions);

        let execution = &mut self
            .blocks
            .get_mut(&block)
            .ok_or_else(|| Error::internal("a reachable block has no analysis state"))?
            .execution;
        Ok(execution.update_input(merged))
    }
}

fn merge_value(
    site: PhiSite,
    lhs: &mut ValueId,
    rhs: ValueId,
    existing_phis: &BTreeMap<PhiSite, PhiDefinition>,
    active_phis: &mut BTreeMap<PhiSite, ValueId>,
    values: &mut ValueContext,
) -> Result<(), Error> {
    if *lhs == rhs {
        return Ok(());
    }
    let result = if let Some(&result) = active_phis.get(&site) {
        result
    } else {
        let result = existing_phis
            .get(&site)
            .map_or_else(|| values.fresh(), |definition| Ok(definition.result))?;
        active_phis.insert(site, result);
        result
    };

    *lhs = result;
    Ok(())
}

fn phi_inputs(
    site: PhiSite,
    contributions: &[(Predecessor, Frame)],
) -> Result<BTreeMap<Predecessor, ValueId>, Error> {
    contributions
        .iter()
        .map(|(predecessor, frame)| {
            let value = frame.value_at(site.position).copied().ok_or_else(|| {
                Error::internal("an active phi input frame lacks its merged slot")
            })?;
            Ok((*predecessor, value))
        })
        .collect()
}

fn synchronize_phi_definitions(
    merged: &Frame,
    contributions: &[(Predecessor, Frame)],
    active_phis: BTreeMap<PhiSite, ValueId>,
) -> Result<BTreeMap<PhiSite, PhiDefinition>, Error> {
    active_phis
        .into_iter()
        .filter_map(|(site, result)| {
            let merged_value = merged.value_at(site.position).copied();
            (merged_value == Some(result)).then_some((site, result))
        })
        .map(|(site, result)| {
            phi_inputs(site, contributions).map(|inputs| {
                let phi_definition = PhiDefinition { result, inputs };
                (site, phi_definition)
            })
        })
        .collect()
}
