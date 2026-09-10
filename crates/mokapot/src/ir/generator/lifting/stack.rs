use super::{FrameOperand, IR, Instruction, JvmStackFrame, MokaIRBuildError, StackOperations};

pub(super) fn lift<OP: FrameOperand>(
    jvm_instruction: &Instruction,
    frame: &mut JvmStackFrame<OP>,
) -> Result<Option<IR<OP>>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use Instruction::*;

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
            IR::Erased
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
