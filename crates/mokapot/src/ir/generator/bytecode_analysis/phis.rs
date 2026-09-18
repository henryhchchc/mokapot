//! Entry-frame merging and complete phi-definition maintenance.

use std::collections::BTreeMap;

use super::{
    analyzer::Analyzer,
    executor::ValueIdAllocator,
    model::{Location, PhiDefinition, PhiSite, Predecessor},
};
use crate::ir::{ValueId, generator::error::Error};

impl Analyzer<'_, '_> {
    pub(super) fn recompute_entry(&mut self, location: Location) -> Result<bool, Error> {
        let location_pc = self.location_pc(location);
        let contributions = self
            .locations
            .get(&location)
            .ok_or_else(|| Error::internal("a reachable block has no location state"))?
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
            let allocator = &mut self.executor.value_id_allocator;
            merged
                .merge_from_with(contribution, |position, lhs, rhs| {
                    merge_value(
                        PhiSite { location, position },
                        lhs,
                        rhs,
                        existing_phis,
                        &mut active_phis,
                        allocator,
                    )
                })
                .map_err(|error| error.at_instruction_if_present(location_pc))?;
        }

        let phi_definitions = synchronize_phi_definitions(&merged, &contributions, active_phis)
            .map_err(|error| error.at_instruction_if_present(location_pc))?;

        self.phi_definitions
            .retain(|site, _| site.location != location);
        self.phi_definitions.extend(phi_definitions);

        let analysis = &mut self
            .locations
            .get_mut(&location)
            .ok_or_else(|| Error::internal("a reachable block has no location state"))?
            .analysis;
        Ok(analysis.update_entry(merged))
    }
}

fn merge_value(
    site: PhiSite,
    lhs: &mut ValueId,
    rhs: ValueId,
    existing_phis: &BTreeMap<PhiSite, PhiDefinition>,
    active_phis: &mut BTreeMap<PhiSite, ValueId>,
    allocator: &mut ValueIdAllocator,
) -> Result<(), Error> {
    if *lhs == rhs {
        return Ok(());
    }
    let result = if let Some(&result) = active_phis.get(&site) {
        result
    } else {
        let result = existing_phis.get(&site).map_or_else(
            || allocator.new_value_id(),
            |definition| Ok(definition.result),
        )?;
        active_phis.insert(site, result);
        result
    };

    *lhs = result;
    Ok(())
}

fn phi_inputs(
    site: PhiSite,
    contributions: &[(Predecessor, super::model::Frame)],
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
    merged: &super::model::Frame,
    contributions: &[(Predecessor, super::model::Frame)],
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
