//! Entry-frame merging and complete phi-definition maintenance.

use std::collections::BTreeMap;

use super::{
    FrameValue,
    analyzer::Analyzer,
    executor::ValueIdAllocator,
    model::{Location, PhiDefinition, PhiKind, PhiSite, Predecessor},
};
use crate::ir::generator::{error::Error, identity::SsaValueId};

impl Analyzer<'_, '_> {
    pub(super) fn recompute_entry(&mut self, location: Location) -> Result<bool, Error> {
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
                .map_err(|error| error.at_instruction_if_present(self.pc(location)))?;
        }

        let phi_definitions = synchronize_phi_definitions(&merged, &contributions, active_phis)
            .map_err(|error| error.at_instruction_if_present(self.pc(location)))?;

        self.phi_definitions
            .retain(|site, _| site.location != location);
        self.phi_definitions.extend(phi_definitions);

        let state = self
            .locations
            .get_mut(&location)
            .ok_or_else(|| Error::internal("a reachable block has no location state"))?;
        let changed = state.entry_frame.as_ref() != Some(&merged);
        state.entry_frame = Some(merged);
        Ok(changed)
    }
}

impl PhiKind {
    const fn of(value: FrameValue) -> Option<Self> {
        match value {
            FrameValue::Ordinary(_) => Some(Self::Ordinary),
            FrameValue::ReturnAddress(_) => Some(Self::ReturnAddress),
            FrameValue::Invalid => None,
        }
    }

    const fn frame_value(self, result: SsaValueId) -> FrameValue {
        match self {
            Self::Ordinary => FrameValue::Ordinary(result),
            Self::ReturnAddress => FrameValue::ReturnAddress(result),
        }
    }
}

fn merge_value(
    site: PhiSite,
    lhs: &mut FrameValue,
    rhs: FrameValue,
    existing_phis: &BTreeMap<PhiSite, PhiDefinition>,
    active_phis: &mut BTreeMap<PhiSite, (SsaValueId, PhiKind)>,
    allocator: &mut ValueIdAllocator,
) -> Result<(), Error> {
    if *lhs == rhs {
        return Ok(());
    }
    let Some(kind) = PhiKind::of(*lhs).filter(|kind| Some(*kind) == PhiKind::of(rhs)) else {
        *lhs = FrameValue::Invalid;
        return Ok(());
    };
    let result = if let Some(&(result, active_kind)) = active_phis.get(&site) {
        if active_kind != kind {
            *lhs = FrameValue::Invalid;
            return Ok(());
        }
        result
    } else {
        let result = existing_phis
            .get(&site)
            .filter(|definition| definition.kind == kind)
            .map_or_else(
                || allocator.new_value_id(),
                |definition| Ok(definition.result),
            )?;
        active_phis.insert(site, (result, kind));
        result
    };

    *lhs = kind.frame_value(result);
    Ok(())
}

fn phi_inputs(
    site: PhiSite,
    kind: PhiKind,
    contributions: &[(Predecessor, super::model::Frame)],
) -> Result<BTreeMap<Predecessor, SsaValueId>, Error> {
    contributions
        .iter()
        .map(|(predecessor, frame)| {
            let value = frame.value_at(site.position).copied().ok_or_else(|| {
                Error::internal("an active phi input frame lacks its merged slot")
            })?;
            if PhiKind::of(value) != Some(kind) {
                return Err(Error::internal(
                    "an active phi input has a different frame-value kind",
                ));
            }
            Ok((*predecessor, value.into_ssa_value_id()?))
        })
        .collect()
}

fn synchronize_phi_definitions(
    merged: &super::model::Frame,
    contributions: &[(Predecessor, super::model::Frame)],
    active_phis: BTreeMap<PhiSite, (SsaValueId, PhiKind)>,
) -> Result<BTreeMap<PhiSite, PhiDefinition>, Error> {
    active_phis
        .into_iter()
        .filter_map(|(site, (result, kind))| {
            let merged_value = merged.value_at(site.position).copied();
            (merged_value == Some(kind.frame_value(result))).then_some((site, (result, kind)))
        })
        .map(|(site, (result, kind))| {
            phi_inputs(site, kind, contributions).map(|inputs| {
                let phi_definition = PhiDefinition {
                    result,
                    kind,
                    inputs,
                };
                (site, phi_definition)
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::generator::bytecode_analysis::jvm::{Position, ValueCategory},
        types::method_descriptor::MethodDescriptor,
    };

    fn frame_with_local(value: Option<FrameValue>) -> super::super::model::Frame {
        let descriptor: MethodDescriptor = "()V".parse().expect("valid descriptor");
        let mut frame = super::super::jvm::Frame::for_method_entry(&descriptor, 1, 0, None, &[])
            .expect("valid frame");
        if let Some(value) = value {
            frame
                .locals
                .set(0, value, ValueCategory::Category1)
                .expect("local exists");
        }
        frame
    }

    #[test]
    fn synchronization_records_typed_inputs_in_predecessor_order() {
        let site = PhiSite {
            location: Location::Unwind,
            position: Position::Local(0),
        };
        let result = SsaValueId::new(9);
        let later = Predecessor::Location(Location::Unwind);
        let contributions = {
            let later_frame = frame_with_local(Some(FrameValue::ReturnAddress(SsaValueId::new(2))));
            let entry_frame = frame_with_local(Some(FrameValue::ReturnAddress(SsaValueId::new(1))));
            vec![(later, later_frame), (Predecessor::Entry, entry_frame)]
        };
        let definitions = synchronize_phi_definitions(
            &frame_with_local(Some(FrameValue::ReturnAddress(result))),
            &contributions,
            BTreeMap::from([(site, (result, PhiKind::ReturnAddress))]),
        )
        .expect("valid phi inputs");
        let definition = &definitions[&site];

        assert_eq!(definition.kind, PhiKind::ReturnAddress);
        assert_eq!(
            definition.inputs.iter().collect::<Vec<_>>(),
            vec![
                (&Predecessor::Entry, &SsaValueId::new(1)),
                (&later, &SsaValueId::new(2)),
            ]
        );
    }

    #[test]
    fn synchronization_prunes_a_phi_when_its_merged_slot_disappears() {
        let site = PhiSite {
            location: Location::Unwind,
            position: Position::Local(0),
        };
        let result = SsaValueId::new(9);
        let definitions = synchronize_phi_definitions(
            &frame_with_local(None),
            &[(Predecessor::Entry, frame_with_local(None))],
            BTreeMap::from([(site, (result, PhiKind::Ordinary))]),
        )
        .expect("a disappeared slot is not an active phi");

        assert!(definitions.is_empty());
    }

    #[test]
    fn synchronization_rejects_a_missing_input_for_an_active_phi() {
        let site = PhiSite {
            location: Location::Unwind,
            position: Position::Local(0),
        };
        let result = SsaValueId::new(9);
        let error = synchronize_phi_definitions(
            &frame_with_local(Some(FrameValue::Ordinary(result))),
            &[(Predecessor::Entry, frame_with_local(None))],
            BTreeMap::from([(site, (result, PhiKind::Ordinary))]),
        )
        .expect_err("an active phi must have every predecessor input");

        assert!(matches!(error, Error::InternalInvariant { .. }));
    }

    #[test]
    fn a_kind_change_allocates_a_new_phi_result() {
        let site = PhiSite {
            location: Location::Unwind,
            position: Position::Local(0),
        };
        let old_result = SsaValueId::new(8);
        let phi_definition = PhiDefinition {
            result: old_result,
            kind: PhiKind::Ordinary,
            inputs: BTreeMap::new(),
        };
        let existing = BTreeMap::from([(site, phi_definition)]);
        let mut active = BTreeMap::new();
        let mut allocator = ValueIdAllocator::default();
        let mut lhs = FrameValue::ReturnAddress(SsaValueId::new(1));

        merge_value(
            site,
            &mut lhs,
            FrameValue::ReturnAddress(SsaValueId::new(2)),
            &existing,
            &mut active,
            &mut allocator,
        )
        .expect("same-kind return addresses merge");

        let (new_result, kind) = active[&site];
        assert_ne!(new_result, old_result);
        assert_eq!(kind, PhiKind::ReturnAddress);
        assert_eq!(lhs, FrameValue::ReturnAddress(new_result));
    }
}
