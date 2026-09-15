use crate::{
    ir::{
        expression::ArrayOperation,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{ValueCategory, ValueCategory::Category1},
                instruction::RegisterInstruction,
                lifting::LiftContext,
            },
        },
    },
    types::field_type::FieldType,
};

impl LiftContext<'_, '_, '_> {
    pub(super) fn array_read(
        &mut self,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let index = self.frame.operand_stack.pop(Category1)?;
        let array_ref = self.frame.operand_stack.pop(Category1)?;
        self.frame.operand_stack.push(value.into(), category)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: ArrayOperation::Read { array_ref, index }.into(),
        })
    }

    pub(super) fn array_write(
        &mut self,
        category: ValueCategory,
    ) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.frame.operand_stack.pop(category)?;
        let index = self.frame.operand_stack.pop(Category1)?;
        let array_ref = self.frame.operand_stack.pop(Category1)?;
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
        let length = self.frame.operand_stack.pop(Category1)?;
        self.frame.operand_stack.push(value.into(), Category1)?;
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
            .map(|_| self.frame.operand_stack.pop(Category1))
            .collect::<Result<_, _>>()?;
        self.frame.operand_stack.push(value.into(), Category1)?;
        let expr = ArrayOperation::NewMultiDim {
            element_type,
            dimensions,
        }
        .into();
        Ok(RegisterInstruction::Definition { value, expr })
    }

    pub(super) fn array_length(&mut self) -> Result<RegisterInstruction, MokaIRBuildError> {
        let value = self.definition_id()?;
        let array_ref = self.frame.operand_stack.pop(Category1)?;
        self.frame.operand_stack.push(value.into(), Category1)?;
        Ok(RegisterInstruction::Definition {
            value,
            expr: ArrayOperation::Length { array_ref }.into(),
        })
    }
}
