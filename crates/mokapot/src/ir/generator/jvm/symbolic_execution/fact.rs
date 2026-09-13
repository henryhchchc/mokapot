use std::collections::BTreeMap;

use crate::ir::{
    control_flow::ControlTransfer,
    generator::{
        identity::SsaValueId,
        jvm::{
            frame::{FrameSlot, JvmStackFrame},
            instruction::RegisterInstruction,
            normalization::{Location, ReturnAddress},
        },
    },
};

/// A stable identity for a frame value merged at a JVM location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) struct MergeIdentity {
    pub location: Location,
    pub slot: FrameSlot,
}

/// The abstract state of an operand during symbolic execution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, derive_more::Display, derive_more::From)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub(crate) enum OperandState {
    Value(#[from] SsaValueId),
    #[display("%return_address")]
    ReturnAddress(#[from] ReturnAddress),
    #[display("%merged")]
    Merged(MergeIdentity),
    #[display("%invalid")]
    Invalid,
}

/// One outgoing edge and its exact symbolic frame.
pub(crate) struct JvmOutgoing {
    pub target: Location,
    pub transfer: ControlTransfer<OperandState>,
    pub frame: JvmStackFrame<OperandState>,
}

/// Completed symbolic-execution facts for one reachable JVM location.
pub(crate) struct AnalyzedLocation {
    pub incoming: JvmStackFrame<OperandState>,
    pub instruction: RegisterInstruction,
    pub outgoing: Vec<JvmOutgoing>,
    pub caught_exception: Option<SsaValueId>,
}

/// Reachable JVM locations and symbolic-execution facts.
pub(crate) struct AnalyzedJvmCfg {
    pub entry_location: Location,
    /// The original frame entering the method.
    pub initial_frame: JvmStackFrame<OperandState>,
    pub locations: BTreeMap<Location, AnalyzedLocation>,
    pub phi_values: BTreeMap<MergeIdentity, SsaValueId>,
    pub this_value: Option<SsaValueId>,
    pub parameter_values: Vec<SsaValueId>,
}
