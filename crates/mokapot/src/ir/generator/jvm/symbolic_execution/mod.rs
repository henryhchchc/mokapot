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

use std::collections::BTreeMap;

pub(crate) use address::NodeAddress;
pub(crate) use fact::{Cfg, Edge, FrameMergeSite, Node, Value};
pub(crate) use instruction::RegisterInstruction;
pub(crate) use subroutine::ReturnAddress;

use crate::{
    ir::generator::{identity::SsaValueId, jvm::frame::Frame},
    jvm::code::MethodBody,
};

/// Mutable state used only while performing symbolic execution.
pub(super) struct Executor<'method> {
    body: &'method MethodBody,
    fallibility: fallibility::Context,
    subroutine_expander: subroutine::Expander,
    definition_ids: BTreeMap<NodeAddress, SsaValueId>,
    caught_exception_ids: BTreeMap<NodeAddress, SsaValueId>,
    value_id_allocator: executor::ValueIdAllocator,
    receiver_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry_addr: NodeAddress,
    initial_frame: Frame<Value>,
}
