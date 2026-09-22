/// A failure while validating or manipulating an abstract JVM frame.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[non_exhaustive]
pub enum Error {
    /// An instruction tried to pop more values than the operand stack holds.
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    /// An instruction grew the operand stack beyond `max_stack`.
    #[error("The stack size exceeds the max stack size")]
    StackOverflow,
    /// An instruction addressed a local slot beyond `max_locals`.
    #[error("The local variable index exceeds the max local variable size")]
    LocalIndexOutOfBounds,
    /// An instruction read a local slot that has never been assigned.
    #[error("The local variable is not initialized")]
    UninitializedLocal,
    /// An instruction read a local slot invalidated by an overlapping write.
    #[error("The local variable is unavailable")]
    UnavailableLocal,
    /// A local or stack value has the wrong category for the requested operation.
    #[error("The slot layout does not match the requested JVM value category")]
    InvalidSlotLayout,
    /// Control-flow inputs have incompatible local or stack shapes.
    #[error("The stack frames have incompatible shapes")]
    IncompatibleFrameShape,
    /// The supplied parameter identities do not match the method descriptor.
    #[error("The number of parameter values does not match the method descriptor")]
    ParameterCountMismatch,
}
