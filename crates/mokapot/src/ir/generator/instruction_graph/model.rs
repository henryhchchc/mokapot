use std::collections::BTreeMap;

use super::{NodeAddress, RegisterInstruction, ReturnAddress};
use crate::ir::{
    control_flow::ControlTransfer,
    generator::{
        identity::SsaValueId,
        instruction_graph::jvm::{Frame, Position},
    },
};

/// A stable identity for a frame value merged at a JVM location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct FrameMergeSite {
    pub addr: NodeAddress,
    pub slot: Position,
}

/// An abstract JVM frame value while constructing the instruction graph.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum Value {
    Ssa(#[from] SsaValueId),
    #[display("%return_address")]
    ReturnAddress(#[from] ReturnAddress),
    #[display("%merged")]
    Merged(FrameMergeSite),
    #[display("%invalid")]
    Invalid,
}

/// One outgoing edge and its exact JVM frame.
pub(crate) struct Edge {
    pub target: NodeAddress,
    pub transfer: ControlTransfer<Value>,
    pub target_frame: Frame<Value>,
}

/// Completed facts for one reachable JVM location.
pub(crate) struct Node {
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
pub(crate) struct Graph {
    pub entry_addr: NodeAddress,
    /// The original frame entering the method.
    pub initial_frame: Frame<Value>,
    pub nodes: BTreeMap<NodeAddress, Node>,
    pub phi_values: BTreeMap<FrameMergeSite, SsaValueId>,
    pub receiver_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
