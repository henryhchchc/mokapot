use super::*;
use crate::jvm::references::ClassRef;

fn instruction_at(method: &MokaIRMethod, pc: ProgramCounter) -> &Operation {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .flat_map(|block| &block.operations)
                .find(|instruction| instruction.id() == id)
        })
        .expect("the source PC must map to an ordinary instruction")
}

fn terminator_at(method: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|id| {
            method
                .blocks()
                .map(|block| &block.terminator)
                .find(|terminator| terminator.id() == id)
        })
        .expect("the source PC must map to a terminator")
}

fn block_containing_instruction(method: &MokaIRMethod, instruction: InstructionId) -> &BasicBlock {
    method
        .blocks()
        .find(|block| {
            block
                .operations
                .iter()
                .any(|candidate| candidate.id() == instruction)
        })
        .expect("the instruction must belong to a block")
}

mod exceptional_state;
mod handler_selection;
mod return_behavior;
mod unwind;
