use crate::{
    ir::{
        expression::ArrayOperation,
        generator::{
            error::MokaIRBuildError,
            jvm::{frame::CATEGORY_1, instruction::RegisterInstruction, lifting::LiftContext},
        },
    },
    types::field_type::FieldType,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn array_read<const SLOT: bool>(
        &mut self,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let index = self.frame.pop_value::<CATEGORY_1>()?;
        let array_ref = self.frame.pop_value::<CATEGORY_1>()?;
        self.frame.push_value::<SLOT>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: ArrayOperation::Read { array_ref, index }.into(),
        })
    }

    pub(super) fn array_write<const SLOT: bool>(
        &mut self,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.pop_value::<SLOT>()?;
        let index = self.frame.pop_value::<CATEGORY_1>()?;
        let array_ref = self.frame.pop_value::<CATEGORY_1>()?;
        Ok(RegisterInstruction::Effect(
            ArrayOperation::Write {
                array_ref,
                index,
                value,
            }
            .into(),
        ))
    }

    pub(super) fn new_array(
        &mut self,
        element_type: FieldType,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let length = self.frame.pop_value::<CATEGORY_1>()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        let expr = ArrayOperation::New {
            element_type,
            length,
        }
        .into();
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn new_multi_array(
        &mut self,
        element_type: FieldType,
        dimension: u8,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let dimensions = (0..dimension)
            .map(|_| self.frame.pop_value::<CATEGORY_1>())
            .collect::<Result<_, _>>()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        let expr = ArrayOperation::NewMultiDim {
            element_type,
            dimensions,
        }
        .into();
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn array_length(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let array_ref = self.frame.pop_value::<CATEGORY_1>()?;
        self.frame.push_value::<CATEGORY_1>(value.into())?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: ArrayOperation::Length { array_ref }.into(),
        })
    }
}
