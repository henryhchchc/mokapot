//! Control flow analysis

use itertools::Itertools;

use crate::jvm::references::ClassRef;
use std::collections::HashSet;

/// The kind of a control transfer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ControlTransfer {
    /// An unconditional control transfer.
    Unconditional,
    /// A conditional contol transfer.
    Conditional,
    /// A control transfer to the exception handler.
    Exception(HashSet<ClassRef>),
    /// A control transfer caused by subroutine return.
    SubroutineReturn,
}

impl std::hash::Hash for ControlTransfer {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        match self {
            ControlTransfer::Unconditional => 0.hash(state),
            ControlTransfer::Conditional => 1.hash(state),
            ControlTransfer::Exception(class_refs) => {
                2.hash(state);
                class_refs
                    .iter()
                    .sorted_unstable_by_key(|&it| &it.binary_name)
                    .for_each(|it| it.hash(state));
            }
            ControlTransfer::SubroutineReturn => 3.hash(state),
        }
    }
}
