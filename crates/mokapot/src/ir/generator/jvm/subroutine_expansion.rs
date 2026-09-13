//! Context-sensitive expansion of legacy JVM `jsr`/`ret` subroutines.

use std::collections::{BTreeMap, BTreeSet};

use crate::jvm::code::ProgramCounter;

use crate::ir::generator::error::MokaIRBuildError;

const EXPANDED_LOCATION_LIMIT: usize = 1_048_576;

/// An interned legacy-subroutine context.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct Context(u32);

impl Context {
    const ROOT: Self = Self(0);
}

/// A private expanded control-flow location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Location {
    Bytecode {
        context: Context,
        pc: ProgramCounter,
    },
    Handler {
        handler_pc: ProgramCounter,
        context: Context,
    },
    Unwind,
}

impl Location {
    pub const fn entry(pc: ProgramCounter) -> Self {
        Self::Bytecode {
            context: Context::ROOT,
            pc,
        }
    }

    pub const fn source_pc(self) -> Option<ProgramCounter> {
        match self {
            Self::Bytecode { pc, .. } => Some(pc),
            Self::Handler { .. } | Self::Unwind => None,
        }
    }

    pub const fn context(self) -> Option<Context> {
        match self {
            Self::Bytecode { context, .. } | Self::Handler { context, .. } => Some(context),
            Self::Unwind => None,
        }
    }
}

/// The exact call activation represented by a JVM `returnAddress` value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct ReturnAddress(Context);

#[cfg(test)]
impl ReturnAddress {
    pub const fn for_test(context: u32) -> Self {
        Self(Context(context))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Activation {
    parent: Context,
    call_site: ProgramCounter,
    target: ProgramCounter,
    continuation: ProgramCounter,
}

#[derive(Debug)]
pub(crate) struct Expander {
    activations: Vec<Option<Activation>>,
    contexts_by_activation: BTreeMap<Activation, Context>,
    expanded_locations: BTreeSet<Location>,
    return_pcs: BTreeMap<Context, ProgramCounter>,
}

impl Expander {
    pub fn new(entry: ProgramCounter) -> Self {
        Self {
            activations: vec![None],
            contexts_by_activation: BTreeMap::new(),
            expanded_locations: BTreeSet::from([Location::entry(entry)]),
            return_pcs: BTreeMap::new(),
        }
    }

    pub fn register_location(&mut self, location: Location) -> Result<Location, MokaIRBuildError> {
        self.expanded_locations.insert(location);
        if self.expanded_locations.len() > EXPANDED_LOCATION_LIMIT {
            return Err(MokaIRBuildError::LegacySubroutineExpansionLimit {
                limit: EXPANDED_LOCATION_LIMIT,
            });
        }
        Ok(location)
    }

    pub fn bytecode_location(
        &mut self,
        pc: ProgramCounter,
        context: Context,
    ) -> Result<Location, MokaIRBuildError> {
        self.register_location(Location::Bytecode { pc, context })
    }

    pub fn handler_location(
        &mut self,
        handler_pc: ProgramCounter,
        context: Context,
    ) -> Result<Location, MokaIRBuildError> {
        self.register_location(Location::Handler {
            handler_pc,
            context,
        })
    }

    pub fn enter_subroutine(
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
            let Some(activation) = self.activation(context)? else {
                break;
            };
            if activation.target == target {
                return Err(MokaIRBuildError::MalformedControlFlow);
            }
            cursor = Some(activation.parent);
        }
        let activation = Activation {
            parent,
            call_site,
            target,
            continuation,
        };
        let context = if let Some(&context) = self.contexts_by_activation.get(&activation) {
            context
        } else {
            let index = u32::try_from(self.activations.len()).map_err(|_| {
                MokaIRBuildError::LegacySubroutineExpansionLimit {
                    limit: EXPANDED_LOCATION_LIMIT,
                }
            })?;
            let context = Context(index);
            self.activations.push(Some(activation));
            self.contexts_by_activation.insert(activation, context);
            context
        };
        Ok((
            self.bytecode_location(target, context)?,
            ReturnAddress(context),
        ))
    }

    pub fn return_from(
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
            let activation = self
                .activation(cursor)?
                .ok_or(MokaIRBuildError::MalformedControlFlow)?;
            if cursor == address.0 {
                match self.return_pcs.get(&cursor).copied() {
                    Some(previous) if previous != return_pc => {
                        return Err(MokaIRBuildError::MalformedControlFlow);
                    }
                    None => {
                        self.return_pcs.insert(cursor, return_pc);
                    }
                    Some(_) => {}
                }
                return self.bytecode_location(activation.continuation, activation.parent);
            }
            cursor = activation.parent;
        }
    }

    fn activation(&self, context: Context) -> Result<Option<Activation>, MokaIRBuildError> {
        self.activations
            .get(usize::try_from(context.0).map_err(|_| MokaIRBuildError::MalformedControlFlow)?)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }
}
