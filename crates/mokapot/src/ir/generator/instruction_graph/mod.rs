//! Constructs a reachable register-form graph from JVM instructions.

mod address;
mod builder;
mod edges;
mod fallibility;
pub(super) mod frame;
mod instruction;
pub(super) mod lifting;
mod model;
mod solver;
mod subroutine;

use std::collections::BTreeMap;

pub(crate) use address::NodeAddress;
pub(crate) use instruction::RegisterInstruction;
pub(crate) use model::{Edge, FrameMergeSite, Graph, Node, Value};
pub(crate) use subroutine::ReturnAddress;

use crate::{
    ir::generator::{identity::SsaValueId, instruction_graph::frame::Frame},
    jvm::{Method, code::MethodBody},
};

pub(super) fn build(method: &Method) -> Result<Graph, crate::ir::generator::error::Error> {
    Builder::for_method(method)?.build()
}

/// Mutable state used while constructing an instruction graph.
pub(super) struct Builder<'method> {
    body: &'method MethodBody,
    fallibility: fallibility::Context,
    subroutine_expander: subroutine::Expander,
    definition_ids: BTreeMap<NodeAddress, SsaValueId>,
    caught_exception_ids: BTreeMap<NodeAddress, SsaValueId>,
    value_id_allocator: builder::ValueIdAllocator,
    receiver_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry_addr: NodeAddress,
    initial_frame: Frame<Value>,
}
