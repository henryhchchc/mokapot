use crate::{
    ir::generator::jvm::lifting::LiftContext,
    ir::{
        expression::Expression,
        generator::{error::MokaIRBuildError, jvm::instruction::RegisterInstruction},
    },
    jvm::ConstantValue,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn constant<const SLOT: bool>(
        &mut self,
        constant: ConstantValue,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        self.frame.push_value::<SLOT>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: Expression::Const(constant),
        })
    }
}
