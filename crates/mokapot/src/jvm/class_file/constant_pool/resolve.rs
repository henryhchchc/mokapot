use std::ops::Deref;

use crate::{
    jvm::{
        ConstantPool, JavaStr,
        class::MethodHandleData,
        constant_pool::{Entry, FieldTypeId, MethodDescriptorId, NameAndType, ReturnTypeId},
        errors::{ParseError, ParsingErrorContext},
        references::{FieldRefData, MethodRefData, ModuleRef},
    },
    types::{
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
        package_name::PackageName,
        reference_type::ReferenceType,
    },
};

use super::{RawConstantPool, RawEntry, Slot};

pub(crate) fn resolve(raw: RawConstantPool<'_>) -> Result<ConstantPoolBuilder, ParseError> {
    let count = raw.count();
    let mut pending = raw.into_slots();
    let utf8_capacity = pending.iter().try_fold(0_usize, |total, slot| {
        let len = match slot {
            Slot::Entry(RawEntry::Utf8(bytes)) => bytes.len(),
            _ => 0,
        };
        total
            .checked_add(len)
            .filter(|&len| len <= isize::MAX.unsigned_abs())
            .context("Constant pool strings exceed allocation limits")
    })?;
    let mut pool = ConstantPoolBuilder::new(count, utf8_capacity);
    for tier in 0..4 {
        for (index, entry) in pending.iter_mut().enumerate() {
            if let Some(raw) = entry.take_if(|entry| entry.resolution_tier() == tier) {
                let index = u16::try_from(index).expect("constant pool count is u16");
                resolve_entry(&mut pool, index, raw).with_context(|error| {
                    format!("Invalid constant pool entry at index {index}: {error}")
                })?;
            }
        }
    }
    Ok(pool)
}

#[cfg(test)]
#[path = "resolve_tests.rs"]
mod tests;

#[derive(Debug, Clone)]
pub(crate) struct ConstantPoolBuilder {
    pool: ConstantPool,
    method_descriptor_indices: Vec<Option<MethodDescriptorId>>,
    field_type_indices: Vec<Option<FieldTypeId>>,
    return_type_indices: Vec<Option<ReturnTypeId>>,
}

impl ConstantPoolBuilder {
    fn new(count: u16, utf8_capacity: usize) -> Self {
        Self {
            pool: ConstantPool::with_slots(count, utf8_capacity),
            method_descriptor_indices: vec![None; usize::from(count)],
            field_type_indices: vec![None; usize::from(count)],
            return_type_indices: vec![None; usize::from(count)],
        }
    }

    pub(crate) fn intern_method_descriptor(
        &mut self,
        source: u16,
    ) -> Result<MethodDescriptorId, ParseError> {
        if let Some(id) = self
            .method_descriptor_indices
            .get(usize::from(source))
            .copied()
            .flatten()
        {
            return Ok(id);
        }
        let value = MethodDescriptor::try_from(self.pool.get_utf8(source)?)
            .context("Invalid method descriptor")?;
        let id = self.pool.push_method_descriptor(value)?;
        *self
            .method_descriptor_indices
            .get_mut(usize::from(source))
            .context("Invalid source index")? = Some(id);
        Ok(id)
    }

    pub(crate) fn intern_field_type(&mut self, source: u16) -> Result<FieldTypeId, ParseError> {
        if let Some(id) = self
            .field_type_indices
            .get(usize::from(source))
            .copied()
            .flatten()
        {
            return Ok(id);
        }
        let value =
            FieldType::try_from(self.pool.get_utf8(source)?).context("Invalid field descriptor")?;
        let id = self.pool.push_field_type(value)?;
        *self
            .field_type_indices
            .get_mut(usize::from(source))
            .context("Invalid source index")? = Some(id);
        Ok(id)
    }

    pub(crate) fn intern_return_type(&mut self, source: u16) -> Result<ReturnTypeId, ParseError> {
        if let Some(id) = self
            .return_type_indices
            .get(usize::from(source))
            .copied()
            .flatten()
        {
            return Ok(id);
        }
        let value = ReturnType::try_from(self.pool.get_utf8(source)?)
            .context("Invalid return descriptor")?;
        let id = self.pool.push_return_type(value)?;
        *self
            .return_type_indices
            .get_mut(usize::from(source))
            .context("Invalid source index")? = Some(id);
        Ok(id)
    }

    pub(crate) fn finish(self) -> ConstantPool {
        self.pool
    }
}

impl Deref for ConstantPoolBuilder {
    type Target = ConstantPool;
    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

fn resolve_entry(
    pool: &mut ConstantPoolBuilder,
    index: u16,
    raw: RawEntry<'_>,
) -> Result<(), ParseError> {
    let entry = match raw {
        RawEntry::Utf8(bytes) => {
            let value = JavaStr::from_cesu8(bytes).context("Invalid JVM modified UTF-8")?;
            pool.pool.insert_utf8_at(index, value);
            return Ok(());
        }
        RawEntry::Integer(value) => Entry::Integer(value.into()),
        RawEntry::Float(value) => Entry::Float(value.into()),
        RawEntry::Long(value) => Entry::Long(value.into()),
        RawEntry::Double(value) => Entry::Double(value.into()),
        RawEntry::Class { name_index } => resolve_class(pool, name_index.into())?,
        RawEntry::String { string_index } => Entry::String(pool.utf8_id(string_index.into())?),
        RawEntry::NameAndType {
            name_index,
            descriptor_index,
        } => {
            let name = pool.utf8_id(name_index.into())?;
            let descriptor_index = descriptor_index.into();
            let descriptor = pool.get_utf8(descriptor_index)?;
            let value = if descriptor.as_bytes().starts_with(b"(") {
                NameAndType::Method(name, pool.intern_method_descriptor(descriptor_index)?)
            } else {
                NameAndType::Field(name, pool.intern_field_type(descriptor_index)?)
            };
            Entry::NameAndType(value)
        }
        RawEntry::MethodType { descriptor_index } => {
            Entry::MethodType(pool.intern_method_descriptor(descriptor_index.into())?)
        }
        RawEntry::Module { name_index } => Entry::Module(ModuleRef {
            name: pool.get_utf8(name_index.into())?.to_owned(),
        }),
        RawEntry::Package { name_index } => Entry::Package(
            PackageName::try_from(pool.get_utf8(name_index.into())?)
                .context("Invalid package name")?,
        ),
        RawEntry::FieldRef {
            class_index,
            name_and_type_index,
        } => Entry::FieldRef(FieldRefData {
            owner: pool.reference_type_id(class_index.into())?,
            name_and_type: pool.field_name_and_type_id(name_and_type_index.into())?,
        }),
        RawEntry::MethodRef {
            class_index,
            name_and_type_index,
        }
        | RawEntry::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        } => {
            let owner = pool.reference_type_id(class_index.into())?;
            let name_and_type = pool.method_name_and_type_id(name_and_type_index.into())?;
            let value = MethodRefData {
                owner,
                name_and_type,
            };
            if matches!(raw, RawEntry::InterfaceMethodRef { .. }) {
                Entry::InterfaceMethodRef(value)
            } else {
                Entry::MethodRef(value)
            }
        }
        RawEntry::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => Entry::Dynamic {
            bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
            name_and_type: pool.field_name_and_type_id(name_and_type_index.into())?,
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => Entry::InvokeDynamic {
            bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
            name_and_type: pool.method_name_and_type_id(name_and_type_index.into())?,
        },
        RawEntry::MethodHandle {
            reference_kind,
            reference_index,
        } => Entry::MethodHandle(resolve_method_handle(
            pool,
            reference_kind,
            reference_index.into(),
        )?),
    };
    pool.pool.insert_resolved_at(index, entry)
}

fn resolve_class(pool: &ConstantPoolBuilder, name_index: u16) -> Result<Entry, ParseError> {
    let name = pool.get_utf8(name_index)?;
    let reference_type = ReferenceType::try_from(name).context("Invalid type reference")?;
    let is_binary_name = !(name.as_bytes().starts_with(b"[")
        || name.as_bytes().starts_with(b"L") && name.as_bytes().ends_with(b";"));
    Ok(Entry::Class {
        reference_type,
        is_binary_name,
    })
}

fn resolve_method_handle(
    pool: &ConstantPoolBuilder,
    kind: u8,
    index: u16,
) -> Result<MethodHandleData, ParseError> {
    Ok(match kind {
        1 => MethodHandleData::GetField(pool.field_ref_id(index)?),
        2 => MethodHandleData::GetStatic(pool.field_ref_id(index)?),
        3 => MethodHandleData::PutField(pool.field_ref_id(index)?),
        4 => MethodHandleData::PutStatic(pool.field_ref_id(index)?),
        5 => MethodHandleData::InvokeVirtual(pool.method_ref_id(index)?),
        6 => MethodHandleData::InvokeStatic(pool.method_or_interface_ref_id(index)?),
        7 => MethodHandleData::InvokeSpecial(pool.method_or_interface_ref_id(index)?),
        8 => MethodHandleData::NewInvokeSpecial(pool.method_ref_id(index)?),
        9 => MethodHandleData::InvokeInterface(pool.interface_method_ref_id(index)?),
        _ => {
            return Err(ParseError::malform(
                "Invalid reference kind in method handle",
            ));
        }
    })
}

impl RawEntry<'_> {
    const fn resolution_tier(&self) -> u8 {
        match self {
            Self::Utf8(_) | Self::Integer(_) | Self::Float(_) | Self::Long(_) | Self::Double(_) => {
                0
            }
            Self::Class { .. }
            | Self::String { .. }
            | Self::NameAndType { .. }
            | Self::MethodType { .. }
            | Self::Module { .. }
            | Self::Package { .. } => 1,
            Self::FieldRef { .. }
            | Self::MethodRef { .. }
            | Self::InterfaceMethodRef { .. }
            | Self::Dynamic { .. }
            | Self::InvokeDynamic { .. } => 2,
            Self::MethodHandle { .. } => 3,
        }
    }
}
