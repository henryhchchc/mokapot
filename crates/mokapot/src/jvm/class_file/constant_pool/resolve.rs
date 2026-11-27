use std::ops::Deref;

use crate::{
    jvm::{
        ConstantPool, ConstantPoolIndex, JavaStr,
        class::MethodHandleData,
        constant_pool::{Entry, FieldTypeId, MethodDescriptorId, NameAndType, ReturnTypeId},
        errors::{ParseError, ParsingErrorContext},
        references::{FieldRefData, MethodRefData},
    },
    types::{
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
        module_name::ModuleName,
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
                resolve_entry(&mut pool, index, raw).map_err(|error| {
                    error.add_context(format!("Invalid constant pool entry at index {index}"))
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
        source: ConstantPoolIndex,
    ) -> Result<MethodDescriptorId, ParseError> {
        if let Some(id) = self
            .method_descriptor_indices
            .get(usize::from(source.get()))
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
            .get_mut(usize::from(source.get()))
            .context("Invalid source index")? = Some(id);
        Ok(id)
    }

    pub(crate) fn intern_field_type(
        &mut self,
        source: ConstantPoolIndex,
    ) -> Result<FieldTypeId, ParseError> {
        if let Some(id) = self
            .field_type_indices
            .get(usize::from(source.get()))
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
            .get_mut(usize::from(source.get()))
            .context("Invalid source index")? = Some(id);
        Ok(id)
    }

    pub(crate) fn intern_return_type(
        &mut self,
        source: ConstantPoolIndex,
    ) -> Result<ReturnTypeId, ParseError> {
        if let Some(id) = self
            .return_type_indices
            .get(usize::from(source.get()))
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
            .get_mut(usize::from(source.get()))
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
        RawEntry::Class { name } => resolve_class(pool, name)?,
        RawEntry::String { string } => Entry::String(pool.utf8_id(string)?),
        RawEntry::NameAndType { name, descriptor } => {
            let name = pool.utf8_id(name)?;
            let value = if pool.get_utf8(descriptor)?.as_bytes().starts_with(b"(") {
                NameAndType::Method(name, pool.intern_method_descriptor(descriptor)?)
            } else {
                NameAndType::Field(name, pool.intern_field_type(descriptor)?)
            };
            Entry::NameAndType(value)
        }
        RawEntry::MethodType { descriptor } => {
            Entry::MethodType(pool.intern_method_descriptor(descriptor)?)
        }
        RawEntry::Module { name } => Entry::Module(
            ModuleName::try_from(pool.get_utf8(name)?).context("Invalid module name")?,
        ),
        RawEntry::Package { name } => Entry::Package(
            PackageName::try_from(pool.get_utf8(name)?).context("Invalid package name")?,
        ),
        RawEntry::FieldRef {
            class,
            name_and_type,
        } => Entry::FieldRef(FieldRefData {
            owner: pool.reference_type_id(class)?,
            name_and_type: pool.field_name_and_type_id(name_and_type)?,
        }),
        RawEntry::MethodRef {
            class,
            name_and_type,
        }
        | RawEntry::InterfaceMethodRef {
            class,
            name_and_type,
        } => {
            let owner = pool.reference_type_id(class)?;
            let name_and_type = pool.method_name_and_type_id(name_and_type)?;
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
            name_and_type,
        } => Entry::Dynamic {
            bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
            name_and_type: pool.field_name_and_type_id(name_and_type)?,
        },
        RawEntry::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type,
        } => Entry::InvokeDynamic {
            bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
            name_and_type: pool.method_name_and_type_id(name_and_type)?,
        },
        RawEntry::MethodHandle {
            reference_kind,
            reference,
        } => Entry::MethodHandle(resolve_method_handle(pool, reference_kind, reference)?),
    };
    pool.pool.insert_resolved_at(index, entry)
}

fn resolve_class(pool: &ConstantPoolBuilder, name: ConstantPoolIndex) -> Result<Entry, ParseError> {
    let name = pool.get_utf8(name)?;
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
    index: ConstantPoolIndex,
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
