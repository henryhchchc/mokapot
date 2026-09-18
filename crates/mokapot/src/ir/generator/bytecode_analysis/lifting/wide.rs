use super::{
    LiftContext,
    ValueCategory::{Category1, Category2},
};
use crate::{
    ir::{Operation, generator::error::Error},
    jvm::code::WideInstruction,
};

impl LiftContext<'_, '_> {
    pub(super) fn lift_wide(
        &mut self,
        instruction: &WideInstruction,
    ) -> Result<Option<Operation>, Error> {
        match instruction {
            WideInstruction::ILoad(idx)
            | WideInstruction::FLoad(idx)
            | WideInstruction::ALoad(idx) => self.load(*idx, Category1),
            WideInstruction::LLoad(idx) | WideInstruction::DLoad(idx) => self.load(*idx, Category2),
            WideInstruction::IStore(idx)
            | WideInstruction::FStore(idx)
            | WideInstruction::AStore(idx) => self.store(*idx, Category1),
            WideInstruction::LStore(idx) | WideInstruction::DStore(idx) => {
                self.store(*idx, Category2)
            }
            WideInstruction::IInc(idx, constant) => self.increment(*idx, *constant),
            WideInstruction::Ret(_) => Err(Error::internal_at(
                self.pc,
                "a wide ret reached non-control lifting",
            )),
        }
    }
}
