use derive_more::Display;
use thiserror::Error;

use super::data_flow::FrameError;
use crate::jvm::code::ProgramCounter;

/// Why JVM bytecode cannot be converted to Moka IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Display)]
#[non_exhaustive]
pub enum ErrorKind {
    /// The method does not have a code body, or the body has no instructions.
    #[display("the method does not have a body or the body is empty")]
    MissingOrEmptyBody,
    /// A control-flow target has no instruction.
    #[display("control-flow target has no instruction")]
    MissingInstruction,
    /// An instruction that must fall through has no following instruction.
    #[display("instruction has no required fallthrough")]
    MissingFallthrough,
    /// A tableswitch has no targets or its case values exceed `i32`.
    #[display("tableswitch has an invalid range")]
    InvalidTableSwitchRange,
    /// A multianewarray has no dimensions or exceeds its array type's rank.
    #[display("multianewarray has invalid dimensions")]
    InvalidMultiArrayDimensions,
    /// Legacy `jsr`/`ret` subroutines are unsupported.
    #[display("legacy jsr/ret subroutines are unsupported")]
    UnsupportedLegacySubroutine,
    /// An operand stack operation had no value to consume.
    #[display("operand stack underflow")]
    StackUnderflow,
    /// An operand stack operation exceeded its maximum size.
    #[display("operand stack overflow")]
    StackOverflow,
    /// A local variable index is outside the method's local-variable array.
    #[display("local variable index is out of bounds")]
    LocalIndexOutOfBounds,
    /// A local variable was read before it was initialized.
    #[display("local variable is uninitialized")]
    UninitializedLocal,
    /// A local variable was invalidated by an overlapping write.
    #[display("local variable is unavailable")]
    UnavailableLocal,
    /// A local or stack value has the wrong category for the requested operation.
    #[display("invalid JVM slot layout")]
    InvalidSlotLayout,
    /// Control-flow paths have incompatible JVM frame shapes.
    #[display("incompatible JVM frame shape")]
    IncompatibleFrameShape,
}

/// An error that occurs when generating Moka IR.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Error)]
#[error("{kind}{}", pc.map(|pc| format!(" at {pc}")).unwrap_or_default())]
pub struct Error {
    /// The relevant bytecode location, such as an instruction, branch target,
    /// or block entry. `None` covers method initialization and synthetic blocks
    /// that have no bytecode location.
    pub pc: Option<ProgramCounter>,
    /// The reason Moka IR generation failed.
    pub kind: ErrorKind,
}

pub(super) trait ResultExt<T> {
    fn at_pc(self, pc: ProgramCounter) -> Result<T, Error>;
}

impl<T, E: Into<ErrorKind>> ResultExt<T> for Result<T, E> {
    fn at_pc(self, pc: ProgramCounter) -> Result<T, Error> {
        self.map_err(|error| Error {
            pc: Some(pc),
            kind: error.into(),
        })
    }
}

impl From<ErrorKind> for Error {
    fn from(kind: ErrorKind) -> Self {
        Self { pc: None, kind }
    }
}

impl From<FrameError> for ErrorKind {
    fn from(error: FrameError) -> Self {
        match error {
            FrameError::StackUnderflow => Self::StackUnderflow,
            FrameError::StackOverflow => Self::StackOverflow,
            FrameError::LocalIndexOutOfBounds => Self::LocalIndexOutOfBounds,
            FrameError::UninitializedLocal => Self::UninitializedLocal,
            FrameError::UnavailableLocal => Self::UnavailableLocal,
            FrameError::InvalidSlotLayout => Self::InvalidSlotLayout,
            FrameError::IncompatibleFrameShape => Self::IncompatibleFrameShape,
        }
    }
}

impl From<FrameError> for Error {
    fn from(error: FrameError) -> Self {
        ErrorKind::from(error).into()
    }
}
