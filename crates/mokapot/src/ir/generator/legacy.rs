//! Context-sensitive normalization support for legacy JVM subroutines.

use std::collections::{BTreeMap, BTreeSet};

use crate::jvm::code::ProgramCounter;

use super::MokaIRBrewingError;

pub(super) const LOCATION_BUDGET: usize = 1_048_576;

/// An interned legacy-subroutine context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) struct ContextId(u32);

impl ContextId {
    const ROOT: Self = Self(0);
}

/// A private expanded control-flow location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(super) enum Location {
    Bytecode {
        context: ContextId,
        pc: ProgramCounter,
    },
    Handler {
        handler_pc: ProgramCounter,
        context: ContextId,
    },
    Unwind,
}

impl Location {
    pub(super) const fn entry(pc: ProgramCounter) -> Self {
        Self::Bytecode {
            context: ContextId::ROOT,
            pc,
        }
    }

    pub(super) const fn source_pc(self) -> Option<ProgramCounter> {
        match self {
            Self::Bytecode { pc, .. } => Some(pc),
            Self::Handler { .. } | Self::Unwind => None,
        }
    }

    pub(super) const fn context(self) -> Option<ContextId> {
        match self {
            Self::Bytecode { context, .. } | Self::Handler { context, .. } => Some(context),
            Self::Unwind => None,
        }
    }
}

/// The exact call activation represented by a JVM `returnAddress` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) struct ReturnAddress(ContextId);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CallFrame {
    parent: ContextId,
    call_site: ProgramCounter,
    target: ProgramCounter,
    continuation: ProgramCounter,
}

#[derive(Debug)]
pub(super) struct Normalizer {
    contexts: Vec<Option<CallFrame>>,
    interned: BTreeMap<CallFrame, ContextId>,
    locations: BTreeSet<Location>,
    returns: BTreeMap<ContextId, ProgramCounter>,
}

impl Normalizer {
    pub(super) fn new(entry: ProgramCounter) -> Self {
        Self {
            contexts: vec![None],
            interned: BTreeMap::new(),
            locations: BTreeSet::from([Location::entry(entry)]),
            returns: BTreeMap::new(),
        }
    }

    pub(super) fn register(&mut self, location: Location) -> Result<Location, MokaIRBrewingError> {
        self.locations.insert(location);
        if self.locations.len() > LOCATION_BUDGET {
            return Err(MokaIRBrewingError::LegacySubroutineExpansionLimit {
                limit: LOCATION_BUDGET,
            });
        }
        Ok(location)
    }

    pub(super) fn bytecode(
        &mut self,
        pc: ProgramCounter,
        context: ContextId,
    ) -> Result<Location, MokaIRBrewingError> {
        self.register(Location::Bytecode { pc, context })
    }

    pub(super) fn handler(
        &mut self,
        handler_pc: ProgramCounter,
        context: ContextId,
    ) -> Result<Location, MokaIRBrewingError> {
        self.register(Location::Handler {
            handler_pc,
            context,
        })
    }

    pub(super) fn enter(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBrewingError> {
        let Location::Bytecode {
            pc: call_site,
            context: parent,
        } = location
        else {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        };
        let mut cursor = Some(parent);
        while let Some(context) = cursor {
            let Some(frame) = self.frame(context)? else {
                break;
            };
            if frame.target == target {
                return Err(MokaIRBrewingError::MalformedControlFlow);
            }
            cursor = Some(frame.parent);
        }
        let frame = CallFrame {
            parent,
            call_site,
            target,
            continuation,
        };
        let context = if let Some(&context) = self.interned.get(&frame) {
            context
        } else {
            let index = u32::try_from(self.contexts.len()).map_err(|_| {
                MokaIRBrewingError::LegacySubroutineExpansionLimit {
                    limit: LOCATION_BUDGET,
                }
            })?;
            let context = ContextId(index);
            self.contexts.push(Some(frame));
            self.interned.insert(frame, context);
            context
        };
        Ok((self.bytecode(target, context)?, ReturnAddress(context)))
    }

    pub(super) fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBrewingError> {
        let Location::Bytecode {
            context: current,
            pc: return_pc,
        } = location
        else {
            return Err(MokaIRBrewingError::MalformedControlFlow);
        };
        let mut cursor = current;
        loop {
            let frame = self
                .frame(cursor)?
                .ok_or(MokaIRBrewingError::MalformedControlFlow)?;
            if cursor == address.0 {
                match self.returns.insert(cursor, return_pc) {
                    Some(previous) if previous != return_pc => {
                        return Err(MokaIRBrewingError::MalformedControlFlow);
                    }
                    Some(_) | None => {}
                }
                return self.bytecode(frame.continuation, frame.parent);
            }
            cursor = frame.parent;
        }
    }

    fn frame(&self, context: ContextId) -> Result<Option<CallFrame>, MokaIRBrewingError> {
        self.contexts
            .get(usize::try_from(context.0).map_err(|_| MokaIRBrewingError::MalformedControlFlow)?)
            .copied()
            .ok_or(MokaIRBrewingError::MalformedControlFlow)
    }
}
