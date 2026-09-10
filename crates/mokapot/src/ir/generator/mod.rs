//! Converts JVM bytecode into completed `MokaIR`.
//!
//! Generation proceeds through four explicit phases:
//!
//! 1. [`jvm_frame_analysis`] produces an immutable set of reachable locations and
//!    incoming JVM frame facts.
//! 2. [`block_formation`] consumes those facts and forms maximal basic blocks.
//! 3. [`ssa`] consumes the frame facts and block layout, replays each block with
//!    exact values, constructs predecessor-indexed phis, and simplifies them.
//! 4. [`emission`] consumes the SSA method, assigns public identities, and emits
//!    the completed [`MokaIRMethod`].
//!
//! The [`lifting`] module contains JVM opcode semantics shared by frame analysis
//! and SSA construction.

mod block_formation;
mod emission;
mod error;
mod identity;
mod jvm_frame;
mod jvm_frame_analysis;
mod lifted_instruction;
mod lifting;
mod normalized_jvm;
mod ssa;

use std::collections::{BTreeMap, BTreeSet};

pub use error::MokaIRBuildError;
use jvm_frame::Entry;
pub use jvm_frame::ExecutionError;

use self::identity::SsaValueId;
use self::jvm_frame_analysis::operand_state::OperandState;
use self::jvm_frame_analysis::{JvmFrameAnalyzer, JvmFrameFacts};
use self::lifted_instruction::LiftedInstruction;
use self::lifting::frame_operand::FrameOperand;
use self::ssa::value::SsaFrameValue;

use self::jvm_frame::JvmStackFrame;
use self::normalized_jvm::{
    Location, NormalizedJvm, Normalizer as LegacyNormalizer, ReturnAddress,
};
use super::{
    BasicBlock, BlockId, EdgeId, InstructionId, MokaIRMethod, Operation as IrOperation,
    OperationKind, Phi, PhiInput, SourceMap, Successor, Terminator, TerminatorKind,
    ValueDefinition, ValueId,
    control_flow::{ControlTransfer, LiftedControlTransfer},
};
use crate::{
    analysis::fixed_point::DataflowProblem,
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

pub(crate) fn generate(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    let frame_facts = JvmFrameAnalyzer::for_method(method)?.run()?;
    let block_layout = block_formation::form(&frame_facts)?;
    let ssa = ssa::construct(frame_facts, block_layout)?;
    emission::emit(ssa)
}

#[cfg(test)]
mod tests;
