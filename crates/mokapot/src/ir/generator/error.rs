use super::data_flow::FrameError;
use crate::jvm::code::ProgramCounter;

/// Why JVM bytecode cannot be converted to Moka IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MalformedControlFlow {
    /// A control-flow target has no instruction.
    #[error("control-flow target at {_0} has no instruction")]
    MissingInstruction(ProgramCounter),
    /// An instruction that must fall through has no following instruction.
    #[error("instruction at {_0} has no required fallthrough")]
    MissingFallthrough(ProgramCounter),
}

/// A well-formed JVM bytecode feature that Moka IR does not support.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
#[non_exhaustive]
pub enum UnsupportedBytecode {
    /// Legacy `jsr`/`ret` subroutines.
    #[display("legacy jsr/ret subroutines are unsupported")]
    LegacySubroutine,
}

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A JVM frame operation failed during dataflow analysis.
    #[error("invalid JVM frame{loc}: {source}", loc = display_pc(pc.as_ref()))]
    InvalidFrame {
        /// The instruction being processed, or `None` for method initialization.
        pc: Option<ProgramCounter>,
        /// The underlying frame failure.
        #[source]
        source: FrameError,
    },
    /// The method does not have a code body.
    #[error("the method does not have a body or the body is empty")]
    MissingOrEmptyBody,
    /// JVM bytecode structure is malformed.
    #[error(transparent)]
    ControlFlow(#[from] MalformedControlFlow),
    /// JVM bytecode uses a feature that Moka IR intentionally does not model.
    #[error("unsupported JVM bytecode at {pc}: {kind}")]
    UnsupportedBytecode {
        /// The unsupported instruction's location.
        pc: ProgramCounter,
        /// The unsupported bytecode feature.
        kind: UnsupportedBytecode,
    },
}

impl Error {
    pub(super) const fn at_pc(self, pc: ProgramCounter) -> Self {
        match self {
            Self::InvalidFrame { pc: None, source } => Self::InvalidFrame {
                pc: Some(pc),
                source,
            },
            error => error,
        }
    }
}

impl From<FrameError> for Error {
    fn from(source: FrameError) -> Self {
        Self::InvalidFrame { pc: None, source }
    }
}

fn display_pc(pc: Option<&ProgramCounter>) -> String {
    pc.map_or_else(String::new, |pc| format!(" at {pc}"))
}
