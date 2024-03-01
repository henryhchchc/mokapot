//! Control flow analysis

use crate::jvm::references::ClassRef;
use std::collections::BTreeSet;

/// The kind of a control transfer.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlTransfer {
    /// An unconditional control transfer.
    Unconditional,
    /// A conditional contol transfer.
    Conditional,
    /// A control transfer to the exception handler.
    Exception(BTreeSet<ClassRef>),
    /// A control transfer caused by subroutine return.
    SubroutineReturn,
}
