//! Local variable types used during attribute parsing.

use std::str::FromStr;

use super::super::{ParseError, ParsingContext, class_element::ClassElement, raw_attributes};
use crate::{
    jvm::{
        code::{LocalVariableId, ProgramCounter},
        errors::ParsingErrorContext,
    },
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

impl ClassElement<'_> for LocalVariableDescAttr {
    type Raw = raw_attributes::LocalVariableInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            start_pc,
            length,
            name_index,
            descriptor_index,
            index,
        } = raw;
        let start_pc = ProgramCounter::from(start_pc);
        let length: u16 = length.into();

        let effective_range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_str(name_index.into())?.to_owned();
        let descriptor = ctx.constant_pool.get_str(descriptor_index.into())?;
        let field_type =
            FieldType::from_str(descriptor).context("Invalid field type descriptor")?;
        let id = LocalVariableId {
            range: effective_range,
            index: index.into(),
        };
        Ok(LocalVariableDescAttr {
            id,
            name,
            field_type,
        })
    }
}

impl ClassElement<'_> for LocalVariableTypeAttr {
    type Raw = raw_attributes::LocalVariableTypeInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            start_pc,
            length,
            name_index,
            signature_index,
            index,
        } = raw;
        let start_pc = ProgramCounter::from(start_pc);
        let length: u16 = length.into();

        let range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_str(name_index.into())?.to_owned();
        let signature = ctx
            .constant_pool
            .get_str(signature_index.into())?
            .to_owned();
        let id = LocalVariableId {
            range,
            index: index.into(),
        };
        Ok(LocalVariableTypeAttr {
            id,
            name,
            signature,
        })
    }
}
