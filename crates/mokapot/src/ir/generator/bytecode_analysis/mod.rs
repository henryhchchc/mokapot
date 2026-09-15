//! Constructs a reachable register-form graph from JVM instructions.

mod address;
mod edges;
mod execution;
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
        generator::{bytecode_analysis::jvm::Frame, error::Error, identity::SsaValueId},
    },
    jvm::{Method, code::MethodBody},
};

pub(super) fn build_node_graph(method: &Method) -> Result<NodeGraph, Error> {
    Executor::for_method(method)?.build_node_graph()
}

/// Symbolic executor that builds a [`NodeGraph`] from JVM instructions.
struct Executor<'method> {
    body: &'method MethodBody,
    fallibility: fallibility::Context,
    subroutine_expander: subroutine::Expander,
    definition_ids: BTreeMap<NodeAddress, SsaValueId>,
    caught_exception_ids: BTreeMap<NodeAddress, SsaValueId>,
    value_id_allocator: execution::ValueIdAllocator,
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
    pub outgoing_edges: Vec<Edge>,
}

impl Node {
    pub(super) fn has_exceptional_exit(&self) -> bool {
        self.outgoing_edges.iter().any(|edge| {
            use ControlTransfer::{Exception, Unwind};
            matches!(edge.transfer, Exception(_) | Unwind)
        })
    }

    /// Whether this node is elided into the block of `next`.
    ///
    /// Blocks are maximal, so only an ordinary operation that neither transfers
    /// control nor can raise, and whose single edge is the fallthrough to
    /// `next`, shares its block with `next`.
    pub(super) fn elides_into(&self, next: NodeAddress) -> bool {
        !self.instruction.is_explicit_transfer()
            && !self.has_exceptional_exit()
            && matches!(self.outgoing_edges.as_slice(), [edge] if edge.target == next)
    }
}

/// A reachable register-form JVM instruction graph.
pub(super) struct NodeGraph {
    pub entry_addr: NodeAddress,
    /// The original frame entering the method.
    pub initial_frame: Frame<Value>,
    pub nodes: BTreeMap<NodeAddress, Node>,
    pub phi_values: BTreeMap<FrameMergeSite, SsaValueId>,
    pub receiver_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
