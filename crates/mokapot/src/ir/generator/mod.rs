mod analysis;
mod assembly;
mod build_model;
mod error;
mod fallibility;
mod jvm_frame;
mod legacy;
mod lifted_instruction;
mod lifting;
mod materialize;
mod merge;
mod remap;
mod scalar;
mod ssa;
mod value;

use std::collections::{BTreeMap, BTreeSet};

pub use error::MokaIRBuildError;
use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::build_model::{
    GeneratedMethod, OutgoingState, PairedFrameValue, PlannedBlock, ScalarArm, ScalarBlock,
    ScalarEntryFrames, next_temp_value,
};
use self::lifted_instruction::LiftedInstruction;
use self::value::{DiscoveryValue, FrameOperand, ProvisionalValueId, ScalarValue};

use self::jvm_frame::JvmStackFrame;
use self::legacy::{Location, Normalizer as LegacyNormalizer, ReturnAddress};
use self::merge::{collect_phi_candidates, unavailable_value_slots};
use self::remap::{remap_expression, remap_transfer};
use super::{
    BasicBlock, BlockId, EdgeId, Instruction as IrInstruction, InstructionId, InstructionKind,
    MokaIRMethod, Phi, PhiInput, SourceMap, Successor, Terminator, TerminatorKind, ValueDefinition,
    ValueId,
    control_flow::{ControlTransfer, LiftedControlTransfer},
    expression::LiftedCondition,
};
use crate::{
    analysis::fixed_point::DataflowProblem,
    ir::control_flow::path_condition::{BooleanVariable, BranchGuard, LiftedValue},
    jvm::{
        ConstantValue, Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

struct MokaIRGenerator<'method> {
    lifted: BTreeMap<Location, LiftedInstruction>,
    outgoing: BTreeMap<Location, Vec<(Location, LiftedControlTransfer<DiscoveryValue>)>>,
    outgoing_frames: BTreeMap<Location, Vec<JvmStackFrame>>,
    value_ids: BTreeMap<Location, ProvisionalValueId>,
    caught_exception_ids: BTreeMap<Location, ProvisionalValueId>,
    method: &'method Method,
    body: &'method MethodBody,
    legacy: LegacyNormalizer,
    discovering: bool,
    next_lifted_value: u32,
    initial_seed: Option<(Location, JvmStackFrame)>,
}

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let generated = MokaIRGenerator::for_method(method)?.generate()?;
    Ok(MokaIRMethod::new(
        method.access_flags,
        method.name.clone(),
        method.descriptor.clone(),
        method.owner.clone(),
        generated.entry,
        generated.blocks,
        generated.source_map,
        generated.this_value,
        generated.parameter_values,
        generated.caught_exceptions,
        generated.value_definitions,
    ))
}

#[cfg(test)]
mod tests;
