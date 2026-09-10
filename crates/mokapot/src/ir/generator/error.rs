use super::ExecutionError;

/// An error that occurs when generating Moka IR.
#[derive(Debug, thiserror::Error)]
pub enum MokaIRBuildError {
    /// An error that occurs when executing bytecode on a JVM frame.
    #[error("Error when executing bytecode on a JVM frame: {0}")]
    ExecutionError(#[from] ExecutionError),
    /// An error that occurs when merging two stack frames.
    #[error("Error when merging two stack frames: {0}")]
    MergeError(ExecutionError),
    /// An error that occurs when a method does not have a body.
    #[error("The method does not have a body")]
    NoMethodBody,
    /// An error that occurs when the method contains malformed control flow.
    #[error("The method contains malformed control flow")]
    MalformedControlFlow,
    /// Legacy subroutine expansion exceeded its deterministic safety budget.
    #[error("legacy subroutine expansion exceeded the {limit}-location budget")]
    LegacySubroutineExpansionLimit {
        /// The maximum number of expanded locations.
        limit: usize,
    },
}
