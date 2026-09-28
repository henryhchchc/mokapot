/// A failure while validating or manipulating an abstract JVM frame.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum FrameError {
    /// An instruction tried to pop more values than the operand stack holds.
    StackUnderflow,
    /// An instruction grew the operand stack beyond `max_stack`.
    StackOverflow,
    /// An instruction addressed a local slot beyond `max_locals`.
    LocalIndexOutOfBounds,
    /// An instruction read a local slot that has never been assigned.
    UninitializedLocal,
    /// An instruction read a local slot invalidated by an overlapping write.
    UnavailableLocal,
    /// A local or stack value has the wrong category for the requested operation.
    InvalidSlotLayout,
    /// Control-flow inputs have incompatible local or stack shapes.
    IncompatibleFrameShape,
}
