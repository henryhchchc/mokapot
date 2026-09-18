//! Worklist orchestration for reachable block analysis.

mod execution;
mod merge;
mod state;

pub(super) use state::{
    CompletedAnalysis, LiftedBlock, Location, PhiDefinition, PhiSite, Predecessor,
};

use std::collections::{BTreeMap, BTreeSet};

use super::{Frame, Position, ScalarGraph, ValueCategory, output, values::ValueContext};
use crate::{
    ir::{
        ValueId,
        generator::{
            bytecode_cfg::{JvmBlockGraph, JvmBlockId},
            error::Error,
        },
    },
    jvm::code::ProgramCounter,
};
use state::{LiftedEdge, LiftedSuccessors, LocationExecution, LocationState};

pub(super) struct Analyzer<'method, 'cfg> {
    pub(super) cfg: &'cfg JvmBlockGraph<'method>,
    pub(super) values: ValueContext,
    pub(super) initial_frame: Frame,
    pub(super) locations: BTreeMap<Location, LocationState>,
    pub(super) phi_definitions: BTreeMap<PhiSite, PhiDefinition>,
    pub(super) caught_exceptions: BTreeMap<JvmBlockId, ValueId>,
}

impl Analyzer<'_, '_> {
    /// Interns the identity of the value caught by the handler entering `block`.
    ///
    /// Every exceptional arm into one handler-entry location must carry the
    /// *same* value: distinct values would merge into a phi at stack position 0
    /// in the handler's entry frame, and `execute_handler` requires that frame
    /// to hold a single stack value.
    pub(super) fn caught_exception(&mut self, block: JvmBlockId) -> Result<ValueId, Error> {
        if let Some(&value) = self.caught_exceptions.get(&block) {
            return Ok(value);
        }
        let value = self.values.fresh()?;
        self.caught_exceptions.insert(block, value);
        Ok(value)
    }

    /// The PC to attribute a diagnostic for `location` to, if it has one.
    ///
    /// A bytecode and a handler location both point at the start of their
    /// block; only the synthetic unwind exit has no PC.
    pub(super) fn location_pc(&self, location: Location) -> Option<ProgramCounter> {
        match location {
            Location::Bytecode(id) | Location::Handler(id) => Some(self.cfg.block(id).start_pc),
            Location::Unwind => None,
        }
    }
}

impl<'method, 'cfg> Analyzer<'method, 'cfg> {
    pub(super) fn new(cfg: &'cfg JvmBlockGraph<'method>) -> Result<Self, Error> {
        let (values, initial_frame) = ValueContext::for_cfg(cfg)?;
        Ok(Self {
            cfg,
            values,
            initial_frame,
            locations: BTreeMap::new(),
            phi_definitions: BTreeMap::new(),
            caught_exceptions: BTreeMap::new(),
        })
    }

    /// Analyzes every reachable location to a fixed point.
    ///
    /// The worklist rests on the invariant documented on [`LocationState`]: a
    /// location's successor targets never change, so its predecessor set only
    /// grows. A changed input frame moves a completed location back to pending.
    pub(super) fn run(mut self) -> Result<ScalarGraph, Error> {
        let entry = Location::Bytecode(self.cfg.entry_block());
        self.locations
            .entry(entry)
            .or_default()
            .contributions
            .insert(Predecessor::Entry, self.initial_frame.clone());
        self.recompute_entry(entry)?;

        let mut pending = BTreeSet::from([entry]);
        while let Some(location) = pending.pop_first() {
            let state = self
                .locations
                .get(&location)
                .expect("a worklist location must have analysis state");
            let LocationExecution::Pending { input } = &state.execution else {
                unreachable!("a worklist location must be pending");
            };
            let input = input.clone();
            let mut block = self.execute(location, input)?;
            let outputs = block.successors.take_output_frames();
            self.locations
                .entry(location)
                .or_default()
                .execution
                .complete(block);
            for (target, frame) in outputs {
                self.locations
                    .entry(target)
                    .or_default()
                    .contributions
                    .insert(Predecessor::Location(location), frame);
                if self.recompute_entry(target)? {
                    pending.insert(target);
                }
            }
        }

        let completed = CompletedAnalysis {
            locations: self.locations,
            phi_definitions: self.phi_definitions,
            receiver_value: self.values.receiver_value,
            parameter_values: self.values.parameter_values,
        };
        output::materialize(completed, entry)
    }
}
