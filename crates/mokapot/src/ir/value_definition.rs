use super::{BlockId, InstructionLocation};

/// Describes where a scalar value is defined.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ValueDefinition {
    /// The receiver of an instance method.
    This,
    /// A method parameter at the given parameter index.
    Parameter(u16),
    /// The exception introduced by a landing-pad block.
    CaughtException(BlockId),
    /// A value produced by an ordinary instruction or block parameter.
    Instruction(InstructionLocation),
}
