use ValueCategory::{Category1, Category2};

use super::{FrameError, LiftContext};
use crate::{
    ir::{
        Operation,
        expression::{Expression, MathOperation},
    },
    jvm::{ConstantVal, code::WideInstruction},
    types::{class_name::ClassName, field_type::ValueCategory},
};

impl<'class> LiftContext<'_, '_> {
    pub fn constant(
        &mut self,
        constant: ConstantVal<'class>,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        self.define_on_stack(category, Expression::Const(constant))
    }

    pub fn increment(
        &mut self,
        idx: u16,
        constant: i32,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.definition_id();
        let base = *self.frame.locals.get(idx, Category1)?;
        self.frame.locals.set(idx, value, Category1)?;
        let expr = MathOperation::Increment(base, constant).into();
        Ok(Some(Operation::Definition { value, expr }))
    }

    pub fn load(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = *self.frame.locals.get(idx, category)?;
        self.frame.stack.push(value, category)?;
        Ok(None)
    }

    pub fn store(
        &mut self,
        idx: u16,
        category: ValueCategory,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        let value = self.frame.stack.pop(category)?;
        self.frame.locals.set(idx, value, category)?;
        Ok(None)
    }

    pub fn new_object(
        &mut self,
        class_name: &'class ClassName,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        self.define_on_stack(Category1, Expression::New(class_name))
    }

    pub fn lift_wide(
        &mut self,
        instruction: WideInstruction,
    ) -> Result<Option<Operation<'class>>, FrameError> {
        #[allow(clippy::enum_glob_use, reason = "exhaustive dispatch")]
        use WideInstruction::*;
        match instruction {
            ILoad(idx) | FLoad(idx) | ALoad(idx) => self.load(idx, Category1),
            LLoad(idx) | DLoad(idx) => self.load(idx, Category2),
            IStore(idx) | FStore(idx) | AStore(idx) => self.store(idx, Category1),
            LStore(idx) | DStore(idx) => self.store(idx, Category2),
            IInc(idx, constant) => self.increment(idx, constant),
            Ret(_) => panic!("wide ret reached non-control lifting"),
        }
    }
}
