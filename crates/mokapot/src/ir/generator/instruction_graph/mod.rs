//! Constructs a reachable register-form graph from JVM instructions.

mod address;
mod builder;
mod edges;
mod fallibility;
mod instruction;
pub(super) mod jvm;
pub(super) mod lifting;
mod solver;
mod subroutine;

use std::collections::BTreeMap;

pub(super) use address::NodeAddress;
pub(super) use instruction::RegisterInstruction;
pub(super) use subroutine::ReturnAddress;

use crate::{
    ir::{
        control_flow::ControlTransfer,
        generator::{error::Error, identity::SsaValueId, instruction_graph::jvm::Frame},
    },
    jvm::{Method, code::MethodBody},
};

pub(super) fn build(method: &Method) -> Result<Graph, Error> {
    Builder::for_method(method)?.build()
}

/// Mutable state used while constructing an instruction graph.
struct Builder<'method> {
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

/// A stable identity for a frame value merged at a JVM location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) struct FrameMergeSite {
    pub addr: NodeAddress,
    pub slot: jvm::Position,
}

/// An abstract JVM frame value while constructing the instruction graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(super) enum Value {
    Ssa(#[from] SsaValueId),
    #[display("%return_address")]
    ReturnAddress(#[from] ReturnAddress),
    #[display("%merged")]
    Merged(FrameMergeSite),
    #[display("%invalid")]
    Invalid,
}

/// One outgoing edge and its exact JVM frame.
pub(super) struct Edge {
    pub target: NodeAddress,
    pub transfer: ControlTransfer<Value>,
    pub target_frame: Frame<Value>,
}

/// Completed facts for one reachable JVM location.
pub(super) struct Node {
    pub incoming_frame: Frame<Value>,
    pub instruction: RegisterInstruction,
    /// Whether executing `instruction` can raise a synchronous exception.
    ///
    /// Such a location always has exceptional outgoing edges and never
    /// coalesces with the next location.
    pub can_throw_synchronously: bool,
    pub outgoing_edges: Vec<Edge>,
    pub caught_exception_value: Option<SsaValueId>,
}

/// A reachable register-form JVM instruction graph.
pub(super) struct Graph {
    pub entry_addr: NodeAddress,
    /// The original frame entering the method.
    pub initial_frame: Frame<Value>,
    pub nodes: BTreeMap<NodeAddress, Node>,
    pub phi_values: BTreeMap<FrameMergeSite, SsaValueId>,
    pub receiver_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
