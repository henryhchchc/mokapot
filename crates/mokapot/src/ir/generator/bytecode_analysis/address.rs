//! Addresses in the context-expanded instruction graph.

use super::subroutine::Context;
use crate::jvm::code::ProgramCounter;

/// A node address in the context-expanded instruction graph.
///
/// Unlike a [`ProgramCounter`], a location also identifies the legacy
/// `jsr`/`ret` activation in which a node executes. Synthetic exception and
/// unwind nodes have no source instruction and therefore no source program
/// counter.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum NodeAddress {
    /// A JVM instruction at `pc`, executing within `context`.
    Bytecode {
        pc: ProgramCounter,
        context: Context,
    },
    /// A synthetic exceptional landing pad for the handler at `handler_pc`.
    ///
    /// This node introduces the caught-exception value and then transfers to
    /// [`NodeAddress::Bytecode`] at `handler_pc`. Keeping it distinct preserves
    /// exceptional-entry semantics when the same bytecode is also normally
    /// reachable.
    Handler {
        handler: ProgramCounter,
        context: Context,
    },
    /// The synthetic exit reached by an exception with no matching handler.
    Unwind,
}

impl NodeAddress {
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
