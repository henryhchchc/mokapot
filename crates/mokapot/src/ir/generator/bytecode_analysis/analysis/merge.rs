//! Entry-frame merging and complete block-parameter maintenance.

use std::collections::BTreeMap;

use super::super::values::ValueContext;
use super::{Analyzer, ParameterSite};
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

        let parameter_definitions = active_parameters
            .into_iter()
            .filter(|(site, result)| merged.value_at(site.position) == Some(result))
            .collect::<BTreeMap<_, _>>();

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
    existing_parameters: &BTreeMap<ParameterSite, ValueId>,
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
            .copied()
            .map_or_else(|| values.fresh(), Ok)?;
        active_parameters.insert(site, result);
        result
    };

    *lhs = result;
    Ok(())
}
