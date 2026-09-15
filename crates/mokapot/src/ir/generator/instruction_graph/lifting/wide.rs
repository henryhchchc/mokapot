use crate::{
    ir::generator::{
        error::Error,
        instruction_graph::{
            RegisterInstruction,
            frame::ValueCategory::{Category1, Category2},
            lifting::Context,
        },
    },
    jvm::code::WideInstruction,
};

impl Context<'_, '_, '_> {
    pub(super) fn lift_wide(
        &mut self,
        instruction: &WideInstruction,
    ) -> Result<RegisterInstruction, Error> {
        match instruction {
            WideInstruction::ILoad(idx)
            | WideInstruction::FLoad(idx)
            | WideInstruction::ALoad(idx) => self.load_unchecked(*idx, Category1),
            WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx) => {
                self.load_unchecked(*idx, Category2)
            }
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx) => self.store(*idx, Category1),
            WideInstruction::LStore(idx) | WideInstruction::DStore(idx) => {
                self.store(*idx, Category2)
            }
            WideInstruction::IInc(idx, constant) => self.increment(*idx, *constant),
            WideInstruction::Ret(idx) => self.subroutine_return(*idx),
        }
    }
}
