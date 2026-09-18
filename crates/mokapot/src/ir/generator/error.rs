use crate::{ir::generator::bytecode_analysis::jvm::FrameError, jvm::code::ProgramCounter};

/// Why JVM bytecode cannot be converted to Moka IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, derive_more::Display)]
#[non_exhaustive]
pub enum MalformedBytecode {
    /// The code attribute contains no entry instruction.
    #[display("the code attribute has no entry instruction")]
    MissingEntry,
    /// A control-flow target has no instruction.
    #[display("a control-flow target has no instruction")]
    MissingInstruction,
    /// An instruction that must fall through has no following instruction.
    #[display("an instruction has no required fallthrough")]
    MissingFallthrough,
    /// An exception-table range is empty, reversed, or not instruction-aligned.
    #[display("an exception-table range is invalid")]
    InvalidExceptionRange,
    /// A `tableswitch` range and jump table have different cardinalities.
    #[display("a tableswitch range does not match its jump table")]
    InvalidTableSwitch,
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
    /// A JVM frame operation failed while processing reachable bytecode.
    #[error("invalid JVM frame{location}: {source}", location = display_location(*pc))]
    InvalidFrame {
        /// The instruction being processed, or `None` for method initialization.
        pc: Option<ProgramCounter>,
        /// The underlying frame failure.
        #[source]
        source: FrameError,
    },
    /// The method does not have a code body.
    #[error("the method does not have a body")]
    NoMethodBody,
    /// JVM bytecode structure is malformed.
    #[error("malformed JVM bytecode{location}: {kind}", location = display_location(*pc))]
    MalformedBytecode {
        /// The relevant bytecode location, when one exists.
        pc: Option<ProgramCounter>,
        /// The invalid bytecode condition.
        kind: MalformedBytecode,
    },
    /// JVM bytecode uses a feature that Moka IR intentionally does not model.
    #[error("unsupported JVM bytecode at instruction {pc}: {kind}")]
    UnsupportedBytecode {
        /// The unsupported instruction's location.
        pc: ProgramCounter,
        /// The unsupported bytecode feature.
        kind: UnsupportedBytecode,
    },
    /// Private construction phases disagreed about an intermediate invariant.
    ///
    /// This indicates a generator defect rather than malformed bytecode.
    #[error("internal IR construction invariant failed{location}: {message}", location = display_location(*pc))]
    InternalInvariant {
        /// The nearest source instruction, when one exists.
        pc: Option<ProgramCounter>,
        /// A stable description suitable for a bug report.
        message: &'static str,
    },
}

impl Error {
    pub(crate) const fn malformed(pc: Option<ProgramCounter>, kind: MalformedBytecode) -> Self {
        Self::MalformedBytecode { pc, kind }
    }

    pub(crate) const fn internal(message: &'static str) -> Self {
        Self::InternalInvariant { pc: None, message }
    }

    pub(crate) const fn internal_at(pc: ProgramCounter, message: &'static str) -> Self {
        Self::InternalInvariant {
            pc: Some(pc),
            message,
        }
    }

    pub(crate) const fn at_instruction(self, pc: ProgramCounter) -> Self {
        match self {
            Self::InvalidFrame { pc: None, source } => Self::InvalidFrame {
                pc: Some(pc),
                source,
            },
            Self::MalformedBytecode { pc: None, kind } => {
                Self::MalformedBytecode { pc: Some(pc), kind }
            }
            Self::InternalInvariant { pc: None, message } => Self::InternalInvariant {
                pc: Some(pc),
                message,
            },
            error => error,
        }
    }

    pub(crate) const fn at_instruction_if_present(self, pc: Option<ProgramCounter>) -> Self {
        match pc {
            Some(pc) => self.at_instruction(pc),
            None => self,
        }
    }
}

impl From<FrameError> for Error {
    fn from(source: FrameError) -> Self {
        Self::InvalidFrame { pc: None, source }
    }
}

fn display_location(pc: Option<ProgramCounter>) -> String {
    pc.map_or_else(String::new, |pc| format!(" at instruction {pc}"))
}
