#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    #[error("The stack size exceeds the max stack size")]
    StackOverflow,
    #[error("The local variable index exceeds the max local variable size")]
    LocalIndexOutOfBounds,
    #[error("The local variable is not initialized")]
    UninitializedLocal,
    #[error("The local variable is unavailable")]
    UnavailableLocal,
    #[error("The slot layout does not match the requested JVM value category")]
    InvalidSlotLayout,
    #[error("The stack frames have incompatible shapes")]
    IncompatibleFrameShape,
    #[error("The number of parameter values does not match the method descriptor")]
    ParameterCountMismatch,
}
