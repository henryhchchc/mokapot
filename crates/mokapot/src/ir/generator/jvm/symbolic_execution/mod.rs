//! Symbolically executes JVM frames to determine reachable states.

mod address;
mod edges;
mod executor;
mod fact;
mod fallibility;
mod instruction;
pub(super) mod lifting;
mod solver;
mod subroutine;

pub(crate) use address::NodeAddress;
pub(super) use executor::Executor;
pub(crate) use fact::{Cfg, Edge, FrameMergeSite, Node, Value};
pub(crate) use instruction::RegisterInstruction;
pub(crate) use subroutine::ReturnAddress;
