use super::{
    BTreeMap, BTreeSet, BlockId, DiscoveryValue, GeneratedMethod, JvmStackFrame,
    LiftedControlTransfer, Location, MokaIRBuildError, MokaIRGenerator, PlannedBlock, ScalarValue,
    collect_phi_candidates, method, next_temp_value, ssa,
};

impl MokaIRGenerator<'_> {
    #[expect(
        clippy::too_many_lines,
        reason = "block partitioning and identity allocation form one invariant-preserving pass"
    )]
    pub(super) fn assemble_blocks(
        mut self,
        facts: &BTreeMap<Location, JvmStackFrame>,
    ) -> Result<GeneratedMethod, MokaIRBuildError> {
        let entry_location = self
            .initial_seed
            .as_ref()
            .map(|(location, _)| *location)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let reachable = self.lifted.keys().copied().collect::<Vec<_>>();
        if reachable.is_empty() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }

        let mut leaders = BTreeSet::from([entry_location]);
        leaders.extend(
            reachable
                .iter()
                .copied()
                .filter(|location| !matches!(location, Location::Bytecode { .. })),
        );
        let mut predecessors: BTreeMap<Location, BTreeSet<Location>> = BTreeMap::new();
        for (&source, arms) in &self.outgoing {
            for (target, _) in arms {
                predecessors.entry(*target).or_default().insert(source);
            }
        }
        leaders.extend(
            predecessors
                .iter()
                .filter(|(_, sources)| sources.len() > 1)
                .map(|(target, _)| *target),
        );

        for (location, instruction) in &self.lifted {
            let arms = self.outgoing.get(location).map_or(
                &[] as &[(Location, LiftedControlTransfer<DiscoveryValue>)],
                Vec::as_slice,
            );
            if instruction.is_explicit_transfer()
                || arms.iter().any(|(_, transfer)| {
                    matches!(
                        transfer,
                        LiftedControlTransfer::Normal
                            | LiftedControlTransfer::Exception(_)
                            | LiftedControlTransfer::Unwind
                    )
                })
            {
                leaders.extend(arms.iter().map(|(target, _)| *target));
            }
        }

        for pair in reachable.windows(2) {
            let [current, next] = pair else {
                unreachable!()
            };
            let instruction = self
                .lifted
                .get(current)
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            let arms = self.outgoing.get(current).map_or(
                &[] as &[(Location, LiftedControlTransfer<DiscoveryValue>)],
                Vec::as_slice,
            );
            let plain_fallthrough = !instruction.is_explicit_transfer()
                && arms.len() == 1
                && arms[0].0 == *next
                && matches!(arms[0].1, LiftedControlTransfer::Unconditional);
            if !plain_fallthrough {
                leaders.insert(*next);
            }
        }
        leaders.retain(|location| self.lifted.contains_key(location));

        let needs_entry_preheader = predecessors
            .get(&entry_location)
            .is_some_and(|sources| !sources.is_empty());
        let block_offset = u32::from(needs_entry_preheader);
        let block_ids = leaders
            .iter()
            .enumerate()
            .map(|(index, location)| {
                u32::try_from(index)
                    .ok()
                    .and_then(|index| index.checked_add(block_offset))
                    .map(|index| (*location, BlockId::new(index)))
                    .ok_or(MokaIRBuildError::MalformedControlFlow)
            })
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let bytecode_entry = *block_ids
            .get(&entry_location)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let entry = if needs_entry_preheader {
            BlockId::new(0)
        } else {
            bytecode_entry
        };

        let mut location_to_block = BTreeMap::new();
        let mut grouped: BTreeMap<BlockId, Vec<Location>> = BTreeMap::new();
        let mut current_block = None;
        for location in reachable {
            if let Some(id) = block_ids.get(&location) {
                current_block = Some(*id);
            }
            let id = current_block.ok_or(MokaIRBuildError::MalformedControlFlow)?;
            location_to_block.insert(location, id);
            grouped.entry(id).or_default().push(location);
        }
        let plans = grouped
            .into_iter()
            .map(|(id, pcs)| PlannedBlock { id, pcs })
            .collect::<Vec<_>>();

        let mut next_temp = self
            .value_ids
            .values()
            .chain(self.caught_exception_ids.values())
            .map(|value| value.index())
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let this_temp = if self
            .method
            .access_flags
            .contains(method::AccessFlags::STATIC)
        {
            None
        } else {
            Some(next_temp_value(&mut next_temp)?)
        };
        let parameter_temps = self
            .method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| next_temp_value(&mut next_temp))
            .collect::<Result<Vec<_>, _>>()?;
        let scalar_this = this_temp.map(ScalarValue::Value);
        let scalar_parameters = parameter_temps
            .iter()
            .copied()
            .map(ScalarValue::Value)
            .collect::<Vec<_>>();
        let initial_scalar_frame = JvmStackFrame::with_inputs(
            &self.method.descriptor,
            self.body.max_locals,
            self.body.max_stack,
            scalar_this,
            &scalar_parameters,
        )?;

        let (entry_frames, phi_blocks) = self.scalar_entry_frames(
            &plans,
            facts,
            bytecode_entry,
            needs_entry_preheader,
            &initial_scalar_frame,
            this_temp,
            &parameter_temps,
            &mut next_temp,
        )?;
        let scalar_blocks =
            self.translate_scalar_blocks(&plans, entry_frames, &location_to_block)?;
        let candidates = collect_phi_candidates(
            &scalar_blocks,
            &phi_blocks,
            needs_entry_preheader.then_some((bytecode_entry, &initial_scalar_frame)),
        )?;
        let simplified =
            ssa::simplify_phis(candidates).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
        self.materialize_scalar_method(
            entry,
            bytecode_entry,
            needs_entry_preheader,
            scalar_blocks,
            &phi_blocks,
            &simplified,
            this_temp,
            &parameter_temps,
        )
    }
}
