use crate::{
    ir::generator::jvm::{instruction::RegisterInstruction, symbolic_execution::Executor},
    jvm::code::Instruction as JVM,
};

impl Executor<'_> {
    pub(super) const fn try_lift_miscellaneous(
        jvm_instruction: &JVM,
    ) -> Option<RegisterInstruction> {
        match jvm_instruction {
            JVM::Nop | JVM::Breakpoint | JVM::ImpDep1 | JVM::ImpDep2 => {
                Some(RegisterInstruction::Erased)
            }
            _ => None,
        }
    }
}
