//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`jvm_frame_analysis`] abstractly executes JVM locals and operand stacks
//!    to determine reachable expanded locations and their incoming states.
//! 2. [`block_formation`] partitions those locations into maximal basic blocks.
//! 3. [`ssa`] replays the blocks with exact values, constructs predecessor-indexed
//!    phis, and simplifies redundant phis.
//! 4. [`emission`] assigns public identities and emits the completed `MokaIR` body.
//!
//! The [`lifting`] module contains JVM opcode semantics shared by frame analysis
//! and SSA construction.

mod block_formation;
mod emission;
mod error;
mod jvm_frame;
mod jvm_frame_analysis;
mod lifted_instruction;
mod lifting;
mod ssa;

use std::collections::{BTreeMap, BTreeSet};

pub use error::MokaIRBuildError;
use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::jvm_frame_analysis::JvmFrameAnalysis;
use self::jvm_frame_analysis::operand_state::OperandState;
use self::lifted_instruction::LiftedInstruction;
use self::lifting::frame_operand::FrameOperand;
use self::ssa::value::{SsaFrameValue, SsaValueId};

use self::jvm_frame::JvmStackFrame;
use self::jvm_frame_analysis::legacy::{Location, ReturnAddress};
use super::{
    BasicBlock, BlockId, EdgeId, InstructionId, MokaIRMethod, Operation as IrOperation,
    OperationKind, Phi, PhiInput, SourceMap, Successor, Terminator, TerminatorKind,
    ValueDefinition, ValueId,
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

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let analysis = JvmFrameAnalysis::for_method(method)?.run()?;
    let block_layout = block_formation::form(analysis)?;
    let ssa = ssa::construct(block_layout)?;
    Ok(emission::emit(ssa)?.into_method(method))
}

#[cfg(test)]
mod tests;
