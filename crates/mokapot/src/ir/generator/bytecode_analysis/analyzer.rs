//! Worklist orchestration for reachable block analysis.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    Executor,
    model::{Location, LocationState, PhiDefinition, PhiSite, Predecessor},
    scalar::ScalarGraph,
};
use crate::{
    ir::generator::{
        bytecode_cfg::{BytecodeCfg, HandlerId},
        error::Error,
        identity::SsaValueId,
    },
    jvm::{Method, code::ProgramCounter},
};

pub(super) struct Analyzer<'method, 'cfg> {
    pub(super) cfg: &'cfg BytecodeCfg,
    pub(super) executor: Executor<'method>,
    pub(super) locations: BTreeMap<Location, LocationState>,
    pub(super) phi_definitions: BTreeMap<PhiSite, PhiDefinition>,
    pub(super) caught_exceptions: BTreeMap<HandlerId, SsaValueId>,
}

pub(super) fn analyze(method: &Method, cfg: &BytecodeCfg) -> Result<ScalarGraph, Error> {
    Analyzer::new(method, cfg)?.run()
}

impl<'method, 'cfg> Analyzer<'method, 'cfg> {
    fn new(method: &'method Method, cfg: &'cfg BytecodeCfg) -> Result<Self, Error> {
        Ok(Self {
            cfg,
            executor: Executor::for_method(method)?,
            locations: BTreeMap::new(),
            phi_definitions: BTreeMap::new(),
            caught_exceptions: BTreeMap::new(),
        })
    }

    /// Analyzes every reachable location to a fixed point.
    ///
    /// The worklist rests on the invariant documented on [`LocationState`]: a
    /// location's successor targets never change, so its predecessor set only
    /// grows and neither an entry frame nor an execution can be revoked.
    fn run(mut self) -> Result<ScalarGraph, Error> {
        let entry = Location::Bytecode(self.cfg.entry_block());
        self.locations
            .entry(entry)
            .or_default()
            .contributions
            .insert(Predecessor::Entry, self.executor.initial_frame.clone());
        self.recompute_entry(entry)?;

        let mut pending = BTreeSet::from([entry]);
        while let Some(location) = pending.pop_first() {
            let input = self
                .locations
                .get(&location)
                .and_then(|state| state.entry_frame.clone())
                .ok_or_else(|| Error::internal("a pending block has no entry frame"))?;
            let block = self.execute(location, input)?;
            let outputs = Self::coalesce_output_frames(&block.successors)?;
            self.locations.entry(location).or_default().execution = Some(block);
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

        self.into_scalar_graph(entry)
    }

    pub(super) fn pc(&self, location: Location) -> Option<ProgramCounter> {
        match location {
            Location::Bytecode(id) => self.cfg.block(id).map(|block| block.start_pc),
            Location::Handler(id) => self.cfg.handler(id).map(|handler| handler.handler_pc),
            Location::Unwind => None,
        }
    }
}
