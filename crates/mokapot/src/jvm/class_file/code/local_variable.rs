//! Local variable types used during attribute parsing.
use super::raw;

use super::super::{ParseError, ResolutionContext, resolve::Resolve};
use crate::jvm::{
    code::{LocalVariableId, LocalVariableTableData, LocalVariableTableEntryData, ProgramCounter},
    constant_pool::{FieldTypeId, Utf8Id},
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

pub(super) fn merge_type(
    table: &mut LocalVariableTableData,
    key: LocalVariableId,
    name: Utf8Id,
    field_type: FieldTypeId,
    pool: &crate::jvm::ConstantPool,
) -> Result<(), ParseError> {
    let entry = entry(table, key, name, pool)?;
    entry.var_type = Some(field_type);
    Ok(())
}

pub(super) fn merge_signature(
    table: &mut LocalVariableTableData,
    key: LocalVariableId,
    name: Utf8Id,
    signature: Utf8Id,
    pool: &crate::jvm::ConstantPool,
) -> Result<(), ParseError> {
    let entry = entry(table, key, name, pool)?;
    entry.signature = Some(signature);
    Ok(())
}

fn entry<'a>(
    table: &'a mut LocalVariableTableData,
    key: LocalVariableId,
    name: Utf8Id,
    pool: &crate::jvm::ConstantPool,
) -> Result<&'a mut LocalVariableTableEntryData, ParseError> {
    let entry = table.entry_data_mut(key);
    if let Some(existing_name) = entry.name
        && pool.utf8(existing_name) != pool.utf8(name)
    {
        return Err(ParseError::malform("Name of local variable does not match"));
    }
    entry.name = Some(name);
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::class_file::constant_pool::{RawConstantPool, RawEntry, resolve};
    use crate::jvm::errors::ParseErrorKind;

    #[test]
    fn merging_local_variable_attributes_accepts_equal_names_and_rejects_conflicts() {
        let bytes = &[0xed, 0xa0, 0x80];
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(bytes),
            RawEntry::Utf8(bytes),
            RawEntry::Utf8(b"other"),
            RawEntry::Utf8(b"I"),
            RawEntry::Utf8(b"J"),
            RawEntry::Utf8(b"TT;"),
        ]);
        let mut pool = resolve(raw).unwrap();
        let names = [1, 2, 3].map(|index| pool.utf8_id(index).unwrap());
        let int = pool.intern_field_type(4).unwrap();
        let long = pool.intern_field_type(5).unwrap();
        let signature = pool.utf8_id(6).unwrap();
        assert_ne!(names[0], names[1]);
        let id = LocalVariableId {
            range: 0.into()..2.into(),
            index: 1,
        };
        for signature_first in [false, true] {
            let mut data = LocalVariableTableData::default();
            if signature_first {
                merge_signature(&mut data, id.clone(), names[0], signature, &pool).unwrap();
                merge_type(&mut data, id.clone(), names[1], int, &pool).unwrap();
            } else {
                merge_type(&mut data, id.clone(), names[0], int, &pool).unwrap();
                merge_signature(&mut data, id.clone(), names[1], signature, &pool).unwrap();
            }
            merge_type(&mut data, id.clone(), names[0], long, &pool).unwrap();
            assert_eq!(
                merge_type(&mut data, id.clone(), names[2], int, &pool)
                    .unwrap_err()
                    .kind(),
                ParseErrorKind::Malformed
            );
            assert_eq!(
                merge_signature(&mut data, id.clone(), names[2], signature, &pool)
                    .unwrap_err()
                    .kind(),
                ParseErrorKind::Malformed
            );
            let entry = data.entry_data_mut(id.clone());
            assert_eq!(pool.utf8(entry.name.unwrap()).as_bytes(), bytes);
            assert_eq!(entry.var_type, Some(long));
            assert_eq!(entry.signature, Some(signature));
        }
    }
}
