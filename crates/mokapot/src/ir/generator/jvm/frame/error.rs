#[derive(Debug, thiserror::Error)]
pub enum ExecutionError {
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    #[error("The stack size exceeds the max stack size")]
    StackOverflow,
    #[error("The local variable index exceeds the max local variable size")]
    LocalLimitExceed,
    #[error("The local variable is not initialized")]
    LocalUninitialized,
    #[error("The local variable is out of scope.")]
    LocalOutOfScope,
    #[error("The stack size mismatch")]
    StackSizeMismatch,
    #[error("The local limit mismatch")]
    LocalLimitMismatch,
    #[error("Value type in the stack or local variable table mismatch")]
    ValueMismatch,
}
