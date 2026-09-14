use crate::{ir::generator::jvm::instruction::RegisterInstruction, jvm::code::Instruction as JVM};

pub(super) const fn try_lift(jvm_instruction: &JVM) -> Option<RegisterInstruction> {
    match jvm_instruction {
        JVM::Nop | JVM::Breakpoint | JVM::ImpDep1 | JVM::ImpDep2 => {
            Some(RegisterInstruction::Erased)
        }
        _ => None,
    }
}
