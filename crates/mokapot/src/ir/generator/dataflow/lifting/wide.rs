use ValueCategory::{Category1, Category2};

use super::{LiftContext, ValueCategory};
use crate::{
    ir::{Operation, generator::error::Error},
    jvm::code::WideInstruction,
};

impl LiftContext<'_, '_> {
    pub fn lift_wide(&mut self, instruction: &WideInstruction) -> Result<Option<Operation>, Error> {
        #[allow(clippy::enum_glob_use, reason = "exhaustive dispatch")]
        use WideInstruction::*;
        match instruction {
            ILoad(idx) | FLoad(idx) | ALoad(idx) => self.load(*idx, Category1),
            LLoad(idx) | DLoad(idx) => self.load(*idx, Category2),
            IStore(idx) | FStore(idx) | AStore(idx) => self.store(*idx, Category1),
            LStore(idx) | DStore(idx) => self.store(*idx, Category2),
            IInc(idx, constant) => self.increment(*idx, *constant),
            Ret(_) => panic!("wide ret reached non-control lifting"),
        }
    }
}
