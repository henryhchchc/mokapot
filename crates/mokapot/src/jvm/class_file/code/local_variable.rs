//! Local variable types used during attribute parsing.
use super::raw;

use super::super::{ParseError, ResolutionContext, resolve::Resolve};
use crate::jvm::{
    class_file::constant_pool::{FieldTypeId, Utf8Id},
    code::{LocalVariableId, ProgramCounter},
    errors::ParsingErrorContext,
};

#[derive(Debug)]
pub(crate) struct LocalVariableDescriptorEntry {
    pub id: LocalVariableId,
    pub name: Utf8Id,
    pub field_type: FieldTypeId,
}

#[derive(Debug)]
pub(crate) struct LocalVariableSignatureEntry {
    pub id: LocalVariableId,
    pub name: Utf8Id,
    pub signature: Utf8Id,
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
        let name = ctx.constant_pool.utf8_id(name_index.into())?;
        let field_type = ctx
            .constant_pool
            .intern_field_type(descriptor_index.into())?;
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
        let name = ctx.constant_pool.utf8_id(name_index.into())?;
        let signature = ctx.constant_pool.utf8_id(signature_index.into())?;
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
