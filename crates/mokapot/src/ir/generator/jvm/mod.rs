//! JVM-specific subroutine expansion, symbolic execution, and instruction lifting.

pub(super) mod frame;
pub(super) mod instruction;
pub(super) mod lifting;
pub(super) mod subroutine;
pub(super) mod symbolic_execution;

use crate::{
    ir::generator::error::MokaIRBuildError,
    jvm::{Method, code::ProgramCounter},
};

pub(super) fn build_symbolic_cfg(
    method: &Method,
) -> Result<symbolic_execution::Cfg, MokaIRBuildError> {
    symbolic_execution::Executor::for_method(method)?.execute()
}

/// A node address in the context-expanded symbolic control-flow graph.
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
        context: subroutine::Context,
    },
    /// A synthetic exceptional landing pad for the handler at `handler_pc`.
    ///
    /// This node introduces the caught-exception value and then transfers to
    /// [`NodeAddress::Bytecode`] at `handler_pc`. Keeping it distinct preserves
    /// exceptional-entry semantics when the same bytecode is also normally
    /// reachable.
    Handler {
        handler: ProgramCounter,
        context: subroutine::Context,
    },
    /// The synthetic exit reached by an exception with no matching handler.
    Unwind,
}

impl NodeAddress {
    pub const fn entry(pc: ProgramCounter) -> Self {
        Self::Bytecode {
            context: subroutine::Context::ROOT,
            pc,
        }
    }

    pub const fn source_pc(self) -> Option<ProgramCounter> {
        match self {
            Self::Bytecode { pc, .. } => Some(pc),
            Self::Handler { .. } | Self::Unwind => None,
        }
    }

    pub const fn context(self) -> Option<subroutine::Context> {
        match self {
            Self::Bytecode { context, .. } | Self::Handler { context, .. } => Some(context),
            Self::Unwind => None,
        }
    }
}
