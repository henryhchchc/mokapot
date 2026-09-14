use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        jvm::{
            frame::{CATEGORY_1, CATEGORY_2},
            instruction::RegisterInstruction,
            lifting::LiftContext,
        },
    },
    jvm::code::WideInstruction,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn lift_wide(
        &mut self,
        instruction: &WideInstruction,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        match instruction {
            WideInstruction::ILoad(idx)
            | WideInstruction::FLoad(idx)
            | WideInstruction::ALoad(idx) => self.load_unchecked::<CATEGORY_1>(*idx),
            WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx) => {
                self.load_unchecked::<CATEGORY_2>(*idx)
            }
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx) => self.store::<CATEGORY_1>(*idx),
            WideInstruction::LStore(idx) | WideInstruction::DStore(idx) => {
                self.store::<CATEGORY_2>(*idx)
            }
            WideInstruction::IInc(idx, constant) => self.increment(*idx, *constant),
            WideInstruction::Ret(idx) => self.subroutine_return(*idx),
        }
    }
}
