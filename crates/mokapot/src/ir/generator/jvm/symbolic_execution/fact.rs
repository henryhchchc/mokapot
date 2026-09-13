use std::collections::BTreeMap;

use crate::ir::{
    control_flow::ControlTransfer,
    generator::{
        identity::SsaValueId,
        jvm::{
            frame::{Frame, Position},
            instruction::RegisterInstruction,
            normalization::{Location, ReturnAddress},
        },
    },
};

/// A stable identity for a frame value merged at a JVM location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct FrameMergeSite {
    pub location: Location,
    pub slot: Position,
}

/// An abstract JVM frame value during symbolic execution.
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

/// One outgoing edge and its exact symbolic frame.
pub(crate) struct Edge {
    pub target: Location,
    pub transfer: ControlTransfer<Value>,
    pub target_frame: Frame<Value>,
}

/// Completed symbolic-execution facts for one reachable JVM location.
pub(crate) struct Node {
    pub incoming_frame: Frame<Value>,
    pub instruction: RegisterInstruction,
    pub outgoing_edges: Vec<Edge>,
    pub caught_exception_value: Option<SsaValueId>,
}

/// Reachable symbolic JVM nodes and their execution facts.
pub(crate) struct Cfg {
    pub entry_location: Location,
    /// The original frame entering the method.
    pub initial_frame: Frame<Value>,
    pub nodes: BTreeMap<Location, Node>,
    pub phi_values: BTreeMap<FrameMergeSite, SsaValueId>,
    pub receiver_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
