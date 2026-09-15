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

pub(super) use address::NodeAddress;
pub(super) use instruction::RegisterInstruction;
pub(super) use model::{Edge, FrameMergeSite, Graph, Node, Value};
pub(super) use subroutine::ReturnAddress;

use crate::{
    ir::generator::{error::Error, identity::SsaValueId, instruction_graph::frame::Frame},
    jvm::{Method, code::MethodBody},
};

pub(super) fn build(method: &Method) -> Result<Graph, Error> {
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
