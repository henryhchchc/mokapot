//! Local variable types used during attribute parsing.

use std::str::FromStr;

use super::super::{ParseError, ParsingContext, class_element::ClassElement, raw_attributes};
use crate::{
    jvm::{
        class::ConstantPool,
        code::LocalVariableId,
        errors::{GenerationError, ParsingErrorContext},
    },
    types::Descriptor,
    types::field_type::FieldType,
};

#[derive(Debug)]
pub(crate) struct LocalVariableDescAttr {
    pub id: LocalVariableId,
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub(crate) struct LocalVariableTypeAttr {
    pub id: LocalVariableId,
    pub name: String,
    pub signature: String,
}

impl ClassElement for LocalVariableDescAttr {
    type Raw = raw_attributes::LocalVariableInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            start_pc,
            length,
            name_index,
            desc_or_signature_idx,
            index,
        } = raw;

        let effective_range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let descriptor = ctx.constant_pool.get_str(desc_or_signature_idx)?;
        let field_type =
            FieldType::from_str(descriptor).context("Invalid field type descriptor")?;
        let id = LocalVariableId {
            effective_range,
            index,
        };
        Ok(LocalVariableDescAttr {
            id,
            name,
            field_type,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let start_pc = self.id.effective_range.start;
        let length = u16::from(self.id.effective_range.end) - u16::from(start_pc);
        let name_index = cp.put_string(self.name)?;
        let desc_or_signature_idx = cp.put_string(self.field_type.descriptor())?;
        let index = self.id.index;
        Ok(Self::Raw {
            start_pc,
            length,
            name_index,
            desc_or_signature_idx,
            index,
        })
    }
}

impl ClassElement for LocalVariableTypeAttr {
    type Raw = raw_attributes::LocalVariableInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            start_pc,
            length,
            name_index,
            desc_or_signature_idx,
            index,
        } = raw;

        let effective_range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let signature = ctx.constant_pool.get_str(desc_or_signature_idx)?.to_owned();
        let id = LocalVariableId {
            effective_range,
            index,
        };
        Ok(LocalVariableTypeAttr {
            id,
            name,
            signature,
        })
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let start_pc = self.id.effective_range.start;
        let length = u16::from(self.id.effective_range.end) - u16::from(start_pc);
        let name_index = cp.put_string(self.name)?;
        let desc_or_signature_idx = cp.put_string(self.signature)?;
        let index = self.id.index;
        Ok(Self::Raw {
            start_pc,
            length,
            name_index,
            desc_or_signature_idx,
            index,
        })
    }
}
