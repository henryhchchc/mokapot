use super::ValueId;

/// An operation on a lock.
#[derive(Debug, Clone, PartialEq, Eq, derive_more::Display)]
pub enum Operation {
    /// Acquires the lock.
    #[display("acquire {_0}")]
    Acquire(ValueId),
    /// Releases the lock.
    #[display("release {_0}")]
    Release(ValueId),
}
