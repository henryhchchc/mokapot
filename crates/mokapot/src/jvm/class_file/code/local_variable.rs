//! Local variable types used during attribute parsing.
use super::raw;

use super::super::{ParseError, ResolutionContext, resolve::Resolve};
use crate::{
    jvm::{
        JavaString,
        code::{LocalVariableId, ProgramCounter},
        errors::ParsingErrorContext,
    },
    types::field_type::FieldType,
};

#[derive(Debug)]
pub(crate) struct LocalVariableDescriptorEntry {
    pub id: LocalVariableId,
    pub name: JavaString,
    pub field_type: FieldType,
}

#[derive(Debug)]
pub(crate) struct LocalVariableSignatureEntry {
    pub id: LocalVariableId,
    pub name: JavaString,
    pub signature: JavaString,
}

impl Resolve for raw::LocalVariableInfo {
    type Output = LocalVariableDescriptorEntry;
    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::LocalVariableInfo {
            start_pc,
            length,
            name_index,
            descriptor_index,
            index,
        } = raw;
        let start_pc = ProgramCounter::from(start_pc);
        let length: u16 = length.into();

        let effective_range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_java_string(name_index.into())?;
        let field_type = FieldType::try_from(ctx.constant_pool.get_utf8(descriptor_index.into())?)
            .context("Invalid field type descriptor")?;
        let id = LocalVariableId {
            range: effective_range,
            index: index.into(),
        };
        Ok(LocalVariableDescriptorEntry {
            id,
            name,
            field_type,
        })
    }
}

impl Resolve for raw::LocalVariableTypeInfo {
    type Output = LocalVariableSignatureEntry;
    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::LocalVariableTypeInfo {
            start_pc,
            length,
            name_index,
            signature_index,
            index,
        } = raw;
        let start_pc = ProgramCounter::from(start_pc);
        let length: u16 = length.into();

        let range = start_pc..(start_pc + length).context("Invalid jump offset")?;
        let name = ctx.constant_pool.get_java_string(name_index.into())?;
        let signature = ctx.constant_pool.get_java_string(signature_index.into())?;
        let id = LocalVariableId {
            range,
            index: index.into(),
        };
        Ok(LocalVariableSignatureEntry {
            id,
            name,
            signature,
        })
    }
}
