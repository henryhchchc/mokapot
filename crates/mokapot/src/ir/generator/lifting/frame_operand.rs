use std::fmt;

use super::super::{ReturnAddress, SsaValueId};

/// An operand that can inhabit a JVM frame while bytecode is lifted.
pub(in crate::ir::generator) trait FrameOperand:
    Clone + Eq + std::hash::Hash + fmt::Display + From<SsaValueId> + From<ReturnAddress>
{
    fn return_address(&self) -> Option<ReturnAddress>;

    fn contains_return_address(&self) -> bool;
}
