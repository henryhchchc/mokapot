//! Forms maximal basic blocks from analyzed JVM locations.

mod block_plan;

pub(in crate::ir::generator) use block_plan::BlockPlan;

use super::{
    BTreeMap, BTreeSet, BlockId, JvmFrameAnalysis, LiftedControlTransfer, Location,
    MokaIRBuildError, OperandState,
};

/// The block layout produced from analyzed JVM locations.
pub(super) struct BlockLayout<'method> {
    pub(super) analysis: JvmFrameAnalysis<'method>,
    pub(super) entry: BlockId,
    pub(super) bytecode_entry: BlockId,
    pub(super) needs_entry_preheader: bool,
    pub(super) plans: Vec<BlockPlan>,
    pub(super) location_to_block: BTreeMap<Location, BlockId>,
}

/// Forms maximal basic blocks from a completed JVM frame analysis.
pub(super) fn form(analysis: JvmFrameAnalysis<'_>) -> Result<BlockLayout<'_>, MokaIRBuildError> {
    analysis.form_blocks()
}

impl<'method> JvmFrameAnalysis<'method> {
    #[expect(
        clippy::too_many_lines,
        reason = "block partitioning and identity allocation form one invariant-preserving pass"
    )]
    fn form_blocks(self) -> Result<BlockLayout<'method>, MokaIRBuildError> {
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
                &[] as &[(Location, LiftedControlTransfer<OperandState>)],
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
                &[] as &[(Location, LiftedControlTransfer<OperandState>)],
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
            .map(|(id, locations)| BlockPlan { id, locations })
            .collect::<Vec<_>>();

        Ok(BlockLayout {
            analysis: self,
            entry,
            bytecode_entry,
            needs_entry_preheader,
            plans,
            location_to_block,
        })
    }
}
