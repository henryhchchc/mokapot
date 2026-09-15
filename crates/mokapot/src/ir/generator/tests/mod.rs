use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    ir::{
        BasicBlock, EdgeId, InstructionId, MokaIRBuildError, MokaIRMethod, Operation,
        OperationKind, Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
        control_flow::ControlTransfer,
    },
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, ProgramCounter},
        method::AccessFlags,
    },
};

/// Builds a `static` method for the generator to lift from the given body.
pub(super) fn method<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> Method
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    crate::tests::method(
        instructions,
        descriptor,
        exception_table,
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    )
}

fn build(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    MokaIRMethod::from_method(method)
}

mod blocks;
mod control_flow;
mod effects;
mod exceptions;
mod legacy;
mod phis;
