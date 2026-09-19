use std::collections::{BTreeMap, HashSet, VecDeque};

use crate::{
    ir::{
        BasicBlock, InstructionLocation, InstructionRef, MokaIRBuildError, MokaIRMethod, Operation,
        Successor, Terminator, ValueDefinition, control_flow::ControlTransfer,
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

pub(super) fn reachable_blocks(ir: &MokaIRMethod) -> Vec<(crate::ir::BlockId, &BasicBlock)> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut pending = VecDeque::from([ir.entry_block()]);
    while let Some(id) = pending.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        let block = ir.block(id).expect("a successor must belong to its method");
        pending.extend(
            block
                .terminator
                .successors()
                .filter_map(Successor::block_target),
        );
        result.push((id, block));
    }
    result
}

mod block_arguments;
mod blocks;
mod control_flow;
mod diagnostics;
mod effects;
mod exceptions;
