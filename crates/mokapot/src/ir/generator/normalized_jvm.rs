//! Context-sensitive normalization support for legacy JVM subroutines.

use std::collections::{BTreeMap, BTreeSet};

use crate::jvm::code::ProgramCounter;

use super::MokaIRBuildError;

pub(crate) const LOCATION_BUDGET: usize = 1_048_576;

/// An interned legacy-subroutine context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct ContextId(u32);

impl ContextId {
    const ROOT: Self = Self(0);
}

/// A private expanded control-flow location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Location {
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
    pub(crate) const fn entry(pc: ProgramCounter) -> Self {
        Self::Bytecode {
            context: ContextId::ROOT,
            pc,
        }
    }

    pub(crate) const fn source_pc(self) -> Option<ProgramCounter> {
        match self {
            Self::Bytecode { pc, .. } => Some(pc),
            Self::Handler { .. } | Self::Unwind => None,
        }
    }

    pub(crate) const fn context(self) -> Option<ContextId> {
        match self {
            Self::Bytecode { context, .. } | Self::Handler { context, .. } => Some(context),
            Self::Unwind => None,
        }
    }
}

/// The exact call activation represented by a JVM `returnAddress` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct ReturnAddress(ContextId);

#[cfg(test)]
impl ReturnAddress {
    pub(crate) const fn for_test(context: u32) -> Self {
        Self(ContextId(context))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct CallFrame {
    parent: ContextId,
    call_site: ProgramCounter,
    target: ProgramCounter,
    continuation: ProgramCounter,
}

#[derive(Debug)]
pub(crate) struct Normalizer {
    contexts: Vec<Option<CallFrame>>,
    interned: BTreeMap<CallFrame, ContextId>,
    locations: BTreeSet<Location>,
    returns: BTreeMap<ContextId, ProgramCounter>,
}

impl Normalizer {
    pub(crate) fn new(entry: ProgramCounter) -> Self {
        Self {
            contexts: vec![None],
            interned: BTreeMap::new(),
            locations: BTreeSet::from([Location::entry(entry)]),
            returns: BTreeMap::new(),
        }
    }

    pub(crate) fn register(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        self.locations.insert(location);
        if self.locations.len() > LOCATION_BUDGET {
            return Err(MokaIRBuildError::LegacySubroutineExpansionLimit {
                limit: LOCATION_BUDGET,
            });
        }
        Ok(location)
    }

    pub(crate) fn bytecode(
        &mut self,
        pc: ProgramCounter,
        context: ContextId,
    ) -> Result<Location, MokaIRBuildError> {
        self.register(Location::Bytecode { pc, context })
    }

    pub(crate) fn handler(
        &mut self,
        handler_pc: ProgramCounter,
        context: ContextId,
    ) -> Result<Location, MokaIRBuildError> {
        self.register(Location::Handler {
            handler_pc,
            context,
        })
    }

    pub(crate) fn enter(
        &mut self,
        location: Location,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(Location, ReturnAddress), MokaIRBuildError> {
        let Location::Bytecode {
            pc: call_site,
            context: parent,
        } = location
        else {
            return Err(MokaIRBuildError::MalformedControlFlow);
        };
        let mut cursor = Some(parent);
        while let Some(context) = cursor {
            let Some(frame) = self.frame(context)? else {
                break;
            };
            if frame.target == target {
                return Err(MokaIRBuildError::MalformedControlFlow);
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
                MokaIRBuildError::LegacySubroutineExpansionLimit {
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

    pub(crate) fn return_from(
        &mut self,
        location: Location,
        address: ReturnAddress,
    ) -> Result<Location, MokaIRBuildError> {
        let Location::Bytecode {
            context: current,
            pc: return_pc,
        } = location
        else {
            return Err(MokaIRBuildError::MalformedControlFlow);
        };
        let mut cursor = current;
        loop {
            let frame = self
                .frame(cursor)?
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if cursor == address.0 {
                match self.returns.get(&cursor).copied() {
                    Some(previous) if previous != return_pc => {
                        return Err(MokaIRBuildError::MalformedControlFlow);
                    }
                    None => {
                        self.returns.insert(cursor, return_pc);
                    }
                    Some(_) => {}
                }
                return self.bytecode(frame.continuation, frame.parent);
            }
            cursor = frame.parent;
        }
    }

    fn frame(&self, context: ContextId) -> Result<Option<CallFrame>, MokaIRBuildError> {
        self.contexts
            .get(usize::try_from(context.0).map_err(|_| MokaIRBuildError::MalformedControlFlow)?)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }
}
