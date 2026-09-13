use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        jvm::{frame::Frame, instruction::RegisterInstruction, symbolic_execution::Value},
    },
    jvm::code::Instruction as JVM,
};

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        Pop | Pop2 | Dup | DupX1 | DupX2 | Dup2 | Dup2X1 | Dup2X2 | Swap => {
            match jvm_instruction {
                Pop => frame.pop()?,
                Pop2 => frame.pop2()?,
                Dup => frame.dup()?,
                DupX1 => frame.dup_x1()?,
                DupX2 => frame.dup_x2()?,
                Dup2 => frame.dup2()?,
                Dup2X1 => frame.dup2_x1()?,
                Dup2X2 => frame.dup2_x2()?,
                Swap => frame.swap()?,
                _ => unreachable!("By outer match arm"),
            }
            RegisterInstruction::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
