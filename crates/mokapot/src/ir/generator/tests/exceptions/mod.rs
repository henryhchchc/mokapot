use super::*;
use crate::jvm::references::ClassRef;

fn terminator_at(method: &MokaIRMethod, pc: ProgramCounter) -> &Terminator {
    method
        .source_map()
        .instructions_at(pc)
        .find_map(|it| {
            matches!(method.instruction(it), Some(InstructionRef::Terminator(_))).then(|| {
                match method.instruction(it) {
                    Some(InstructionRef::Terminator(terminator)) => terminator,
                    _ => panic!(),
                }
            })
        })
        .expect("the source PC must map to a terminator")
}

fn block_containing_instruction(
    method: &MokaIRMethod,
    location: InstructionLocation,
) -> &BasicBlock {
    let block = match location {
        InstructionLocation::BlockParameter { block, .. }
        | InstructionLocation::Operation { block, .. }
        | InstructionLocation::Terminator { block } => block,
    };
    method
        .block(block)
        .expect("the instruction must belong to a block")
}

mod exceptional_state;
mod handler_selection;
mod return_behavior;
mod unwind;
