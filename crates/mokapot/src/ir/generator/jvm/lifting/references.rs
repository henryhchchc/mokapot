use crate::{
    ir::{
        expression::{Expression, LockOperation},
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::CATEGORY_1, instruction::RegisterInstruction, lifting::LiftContext,
                symbolic_execution::Value,
            },
        },
    },
    jvm::references::ClassRef,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn new_object(
        &mut self,
        class: &ClassRef,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: Expression::New(class.clone()),
        })
    }

    pub(super) fn monitor(
        &mut self,
        operation: impl FnOnce(Value) -> LockOperation<Value>,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let object_ref = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Effect(operation(object_ref).into()))
    }
}
