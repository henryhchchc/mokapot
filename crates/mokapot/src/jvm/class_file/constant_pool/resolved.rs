//! Eagerly resolved JVM constant-pool values.

use std::{fmt, num::NonZeroU16, ops::Range};

use crate::{
    jvm::{
        ConstantValueView, JavaStr,
        class::{MethodHandleData, MethodHandleView},
        class_file::constant_pool::storage::{PoolStorage, Slot},
        class_file::constant_pool::{RawConstantPool, RawEntry},
        errors::{ParseError, ParsingErrorContext},
        references::{FieldRefData, FieldRefView, MethodRefData, MethodRefView, ModuleRef},
    },
    types::{
        class_name::ClassName,
        field_type::FieldType,
        method_descriptor::{MethodDescriptor, ReturnType},
        package_name::PackageName,
        reference_type::ReferenceType,
    },
};

/// An immutable constant pool whose entries have been resolved to typed values.
#[derive(Clone, PartialEq)]
pub struct ConstantPool {
    inner: PoolStorage<Entry>,
    utf8_bytes: Vec<u8>,
    method_descriptors: ValueCache<MethodDescriptor>,
    field_types: ValueCache<FieldType>,
    return_types: ValueCache<ReturnType>,
}

impl fmt::Debug for ConstantPool {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ConstantPool")
            .field("inner", &self.inner)
            .field("utf8_bytes_len", &self.utf8_bytes.len())
            .field("method_descriptors", &self.method_descriptors)
            .field("field_types", &self.field_types)
            .field("return_types", &self.return_types)
            .finish()
    }
}

#[derive(Debug, Clone, PartialEq)]
struct ValueCache<T> {
    values: Vec<T>,
    indices: Vec<Option<u16>>,
}

impl<T> ValueCache<T> {
    fn new(count: u16) -> Self {
        Self {
            values: Vec::new(),
            indices: vec![None; usize::from(count)],
        }
    }
    fn lookup(&self, source: u16) -> Option<u16> {
        self.indices.get(usize::from(source)).copied().flatten()
    }
    fn insert(&mut self, source: u16, value: T) -> Result<u16, ParseError> {
        let slot = self
            .indices
            .get_mut(usize::from(source))
            .context("Invalid source index")?;
        let id = u16::try_from(self.values.len())
            .map_err(|_| ParseError::malform("Too many cached values"))?;
        self.values.push(value);
        *slot = Some(id);
        Ok(id)
    }
    fn get(&self, id: u16) -> &T {
        &self.values[usize::from(id)]
    }
    fn finish(&mut self) {
        self.indices = Vec::new();
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Utf8Id(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodDescriptorId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClassId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReferenceTypeId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PackageId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModuleId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodHandleId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodNameAndTypeId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldNameAndTypeId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldTypeId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReturnTypeId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstantValueId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldRefId(NonZeroU16);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodRefId(NonZeroU16);
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct InvokeDynamicId(NonZeroU16);
/// A resolved constant-pool entry.
#[derive(Debug, Clone, PartialEq)]
enum Entry {
    /// A string in JVM modified UTF-8 encoding.
    Utf8(Range<usize>),
    /// An integer.
    Integer(i32),
    /// A float.
    Float(f32),
    /// A long.
    Long(i64),
    /// A double.
    Double(f64),
    /// A class, interface, or array type.
    Class {
        /// The parsed class, interface, or array type.
        reference_type: ReferenceType,
        /// Whether the source name is a binary name accepted in class-name contexts.
        is_binary_name: bool,
    },
    /// A string literal.
    String(Utf8Id),
    /// A field reference.
    FieldRef(FieldRefData),
    /// A method reference.
    MethodRef(MethodRefData),
    /// An interface method reference.
    InterfaceMethodRef(MethodRefData),
    /// A field or method name and descriptor.
    NameAndType(NameAndType),
    /// A method handle.
    MethodHandle(MethodHandleData),
    /// A method descriptor.
    MethodType(MethodDescriptorId),
    /// A dynamic constant, retaining its bootstrap-table index.
    Dynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        name_and_type: FieldNameAndTypeId,
    },
    /// A dynamic call site, retaining its bootstrap-table index.
    InvokeDynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        name_and_type: MethodNameAndTypeId,
    },
    /// A module reference.
    Module(ModuleRef),
    /// A package name.
    Package(PackageName),
}

/// A resolved field or method name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
enum NameAndType {
    /// A field name and type.
    Field(Utf8Id, FieldTypeId),
    /// A method name and descriptor.
    Method(Utf8Id, MethodDescriptorId),
}

impl Entry {
    fn type_mismatch(&self, expected: &'static str) -> ParseError {
        ParseError::malform(format!(
            "Mismatched constant pool type. Expected: {expected} but got {}.",
            self.constant_kind()
        ))
    }

    /// Returns the JVM constant kind.
    #[must_use]
    pub const fn constant_kind(&self) -> &'static str {
        match self {
            Self::Utf8(_) => "CONSTANT_Utf8",
            Self::Integer(_) => "CONSTANT_Integer",
            Self::Float(_) => "CONSTANT_Float",
            Self::Long(_) => "CONSTANT_Long",
            Self::Double(_) => "CONSTANT_Double",
            Self::Class { .. } => "CONSTANT_Class",
            Self::String(_) => "CONSTANT_String",
            Self::FieldRef(_) => "CONSTANT_Fieldref",
            Self::MethodRef(_) => "CONSTANT_Methodref",
            Self::InterfaceMethodRef(_) => "CONSTANT_InterfaceMethodref",
            Self::NameAndType(_) => "CONSTANT_NameAndType",
            Self::MethodHandle(_) => "CONSTANT_MethodHandle",
            Self::MethodType(_) => "CONSTANT_MethodType",
            Self::Dynamic { .. } => "CONSTANT_Dynamic",
            Self::InvokeDynamic { .. } => "CONSTANT_InvokeDynamic",
            Self::Module(_) => "CONSTANT_Module",
            Self::Package(_) => "CONSTANT_Package",
        }
    }
}

impl ConstantPool {
    pub(crate) fn field_ref_id(&self, index: u16) -> Result<FieldRefId, ParseError> {
        match self.entry(index)? {
            Entry::FieldRef(_) => Ok(FieldRefId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("Fieldref")),
        }
    }
    pub(crate) fn method_ref_id(&self, index: u16) -> Result<MethodRefId, ParseError> {
        match self.entry(index)? {
            Entry::MethodRef(_) => Ok(MethodRefId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("Methodref")),
        }
    }
    pub(crate) fn interface_method_ref_id(&self, index: u16) -> Result<MethodRefId, ParseError> {
        match self.entry(index)? {
            Entry::InterfaceMethodRef(_) => Ok(MethodRefId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("InterfaceMethodref")),
        }
    }
    pub(crate) fn method_or_interface_ref_id(&self, index: u16) -> Result<MethodRefId, ParseError> {
        match self.entry(index)? {
            Entry::MethodRef(_) | Entry::InterfaceMethodRef(_) => Ok(MethodRefId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("Methodref | InterfaceMethodref")),
        }
    }
    pub(crate) fn invoke_dynamic_id(&self, index: u16) -> Result<InvokeDynamicId, ParseError> {
        match self.entry(index)? {
            Entry::InvokeDynamic { .. } => Ok(InvokeDynamicId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("InvokeDynamic")),
        }
    }
    pub(crate) fn field_ref(&self, id: FieldRefId) -> FieldRefView<'_> {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::FieldRef(value) => FieldRefView::new(self, value),
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn method_ref(&self, id: MethodRefId) -> MethodRefView<'_> {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::MethodRef(value) | Entry::InterfaceMethodRef(value) => {
                MethodRefView::new(self, value)
            }
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn invoke_dynamic(&self, id: InvokeDynamicId) -> (u16, &JavaStr, &MethodDescriptor) {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type,
            } => {
                let (name, descriptor) = self.method_name_and_type(*name_and_type);
                (*bootstrap_method_attr_index, name, descriptor)
            }
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn reference_type_id(&self, index: u16) -> Result<ReferenceTypeId, ParseError> {
        match self.entry(index)? {
            Entry::Class { .. } => Ok(ReferenceTypeId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("Class")),
        }
    }
    pub(crate) fn reference_type(&self, id: ReferenceTypeId) -> &ReferenceType {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::Class { reference_type, .. } => reference_type,
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn class_id(&self, index: u16) -> Result<ClassId, ParseError> {
        match self.entry(index)? {
            Entry::Class {
                reference_type: ReferenceType::Class(_),
                is_binary_name: true,
            } => Ok(ClassId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("ClassName")),
        }
    }
    pub(crate) fn class_name(&self, id: ClassId) -> &ClassName {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::Class {
                reference_type: ReferenceType::Class(value),
                is_binary_name: true,
            } => value,
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn package_id(&self, index: u16) -> Result<PackageId, ParseError> {
        match self.entry(index)? {
            Entry::Package(_) => Ok(PackageId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("PackageName")),
        }
    }
    pub(crate) fn package_name(&self, id: PackageId) -> &PackageName {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::Package(value) => value,
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn module_id(&self, index: u16) -> Result<ModuleId, ParseError> {
        match self.entry(index)? {
            Entry::Module(_) => Ok(ModuleId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("ModuleRef")),
        }
    }
    pub(crate) fn module_ref(&self, id: ModuleId) -> &ModuleRef {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::Module(value) => value,
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn method_handle_id(&self, index: u16) -> Result<MethodHandleId, ParseError> {
        match self.entry(index)? {
            Entry::MethodHandle(_) => Ok(MethodHandleId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("MethodHandle")),
        }
    }
    pub(crate) fn method_handle(&self, id: MethodHandleId) -> MethodHandleView<'_> {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::MethodHandle(value) => value.view(self),
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn method_name_and_type_id(
        &self,
        index: u16,
    ) -> Result<MethodNameAndTypeId, ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Method(..)) => Ok(MethodNameAndTypeId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("method name and type")),
        }
    }
    pub(crate) fn method_name_and_type(
        &self,
        id: MethodNameAndTypeId,
    ) -> (&JavaStr, &MethodDescriptor) {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::NameAndType(NameAndType::Method(name, descriptor)) => {
                (self.utf8(*name), self.method_descriptor(*descriptor))
            }
            _ => unreachable!("validated entry kind"),
        }
    }
    pub(crate) fn intern_field_type(&mut self, index: u16) -> Result<FieldTypeId, ParseError> {
        if let Some(id) = self.field_types.lookup(index) {
            return Ok(FieldTypeId(id));
        }
        let value =
            FieldType::try_from(self.get_utf8(index)?).context("Invalid field descriptor")?;
        self.field_types.insert(index, value).map(FieldTypeId)
    }
    pub(crate) fn field_type(&self, id: FieldTypeId) -> &FieldType {
        self.field_types.get(id.0)
    }
    pub(crate) fn intern_return_type(&mut self, index: u16) -> Result<ReturnTypeId, ParseError> {
        if let Some(id) = self.return_types.lookup(index) {
            return Ok(ReturnTypeId(id));
        }
        let value =
            ReturnType::try_from(self.get_utf8(index)?).context("Invalid return descriptor")?;
        self.return_types.insert(index, value).map(ReturnTypeId)
    }
    pub(crate) fn return_type(&self, id: ReturnTypeId) -> &ReturnType {
        self.return_types.get(id.0)
    }
    pub(crate) fn constant_value_id(&self, index: u16) -> Result<ConstantValueId, ParseError> {
        self.get_constant_value(index)?;
        Ok(ConstantValueId(
            NonZeroU16::new(index).expect("validated nonzero pool index"),
        ))
    }
    pub(crate) fn constant_value(&self, id: ConstantValueId) -> ConstantValueView<'_> {
        self.get_constant_value(id.0.get())
            .expect("validated constant pool ID")
    }
    pub(crate) fn get_utf8(&self, index: u16) -> Result<&JavaStr, ParseError> {
        match self.entry(index)? {
            Entry::Utf8(range) => {
                let bytes = &self.utf8_bytes[range.clone()];
                // SAFETY: append_utf8 records the exact bounds of a validated JavaStr.
                Ok(unsafe { JavaStr::from_cesu8_unchecked(bytes) })
            }
            entry => Err(entry.type_mismatch("Utf8")),
        }
    }

    pub(crate) fn utf8_id(&self, index: u16) -> Result<Utf8Id, ParseError> {
        self.get_utf8(index)?;
        Ok(Utf8Id(
            NonZeroU16::new(index).expect("validated nonzero pool index"),
        ))
    }

    pub(crate) fn utf8(&self, id: Utf8Id) -> &JavaStr {
        self.get_utf8(id.0.get()).expect("validated UTF-8 pool ID")
    }

    pub(crate) fn method_descriptor(&self, id: MethodDescriptorId) -> &MethodDescriptor {
        self.method_descriptors.get(id.0)
    }

    pub(crate) fn intern_method_descriptor(
        &mut self,
        index: u16,
    ) -> Result<MethodDescriptorId, ParseError> {
        if let Some(id) = self.method_descriptors.lookup(index) {
            return Ok(MethodDescriptorId(id));
        }
        let value = MethodDescriptor::try_from(self.get_utf8(index)?)
            .context("Invalid method descriptor")?;
        self.method_descriptors
            .insert(index, value)
            .map(MethodDescriptorId)
    }

    pub(crate) fn finish(mut self) -> Self {
        self.method_descriptors.finish();
        self.field_types.finish();
        self.return_types.finish();
        self
    }

    /// Resolves all entries, preserving indices and reserved slots.
    /// # Errors
    /// Returns an error for invalid references, tags, names, descriptors, or handle kinds.
    pub fn from_raw(raw: RawConstantPool<'_>) -> Result<Self, ParseError> {
        let count = raw.count();
        let mut pending = raw.inner.into_slots();
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
        let mut pool = Self {
            inner: PoolStorage::with_padding(count),
            utf8_bytes: Vec::with_capacity(utf8_capacity),
            method_descriptors: ValueCache::new(count),
            field_types: ValueCache::new(count),
            return_types: ValueCache::new(count),
        };
        // Each tier depends only on earlier tiers, regardless of source entry order.
        for tier in 0..4 {
            for (index, entry) in pending.iter_mut().enumerate() {
                if let Some(raw) = entry.take_if(|entry| entry.resolution_tier() == tier) {
                    pool.inner.as_mut_slice()[index] =
                        Slot::Entry(pool.resolve_entry(raw).with_context(|error| {
                            format!("Invalid constant pool entry at index {index}: {error}")
                        })?);
                }
            }
        }
        Ok(pool)
    }

    /// Gets an entry, or `None` for an invalid index or reserved slot.
    #[must_use]
    fn get_entry(&self, index: u16) -> Option<&Entry> {
        self.inner.get_entry(index)
    }

    fn entry(&self, index: u16) -> Result<&Entry, ParseError> {
        self.get_entry(index).context("Invalid constant pool index")
    }

    fn field_name_and_type_id(&self, index: u16) -> Result<FieldNameAndTypeId, ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Field(..)) => Ok(FieldNameAndTypeId(
                NonZeroU16::new(index).expect("validated nonzero index"),
            )),
            entry => Err(entry.type_mismatch("NameAndType with a field descriptor")),
        }
    }

    pub(crate) fn field_name_and_type(&self, id: FieldNameAndTypeId) -> (&JavaStr, &FieldType) {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::NameAndType(NameAndType::Field(name, ty)) => {
                (self.utf8(*name), self.field_type(*ty))
            }
            _ => unreachable!("validated entry kind"),
        }
    }

    fn get_constant_value(&self, index: u16) -> Result<ConstantValueView<'_>, ParseError> {
        Ok(match self.entry(index)? {
            Entry::Integer(value) => ConstantValueView::Integer(*value),
            Entry::Float(value) => ConstantValueView::Float(*value),
            Entry::Long(value) => ConstantValueView::Long(*value),
            Entry::Double(value) => ConstantValueView::Double(*value),
            Entry::String(value) => ConstantValueView::String(self.utf8(*value)),
            Entry::Class { reference_type, .. } => ConstantValueView::Class(reference_type),
            Entry::MethodType(value) => {
                ConstantValueView::MethodType(self.method_descriptor(*value))
            }
            Entry::MethodHandle(value) => ConstantValueView::Handle(value.view(self)),
            Entry::Dynamic {
                bootstrap_method_attr_index,
                name_and_type,
            } => {
                let (name, ty) = self.field_name_and_type(*name_and_type);
                ConstantValueView::Dynamic(*bootstrap_method_attr_index, name, ty)
            }
            entry => return Err(entry.type_mismatch("Loadable constant")),
        })
    }

    fn resolve_entry(&mut self, raw: RawEntry<'_>) -> Result<Entry, ParseError> {
        Ok(match raw {
            RawEntry::Utf8(bytes) => {
                let value = JavaStr::from_cesu8(bytes).context("Invalid JVM modified UTF-8")?;
                Entry::Utf8(self.append_utf8(value))
            }
            RawEntry::Integer(value) => Entry::Integer(value.into()),
            RawEntry::Float(value) => Entry::Float(value.into()),
            RawEntry::Long(value) => Entry::Long(value.into()),
            RawEntry::Double(value) => Entry::Double(value.into()),
            RawEntry::Class { name_index } => self.resolve_class(name_index.into())?,
            RawEntry::String { string_index } => Entry::String(self.utf8_id(string_index.into())?),
            RawEntry::NameAndType {
                name_index,
                descriptor_index,
            } => Entry::NameAndType(
                self.resolve_name_and_type(name_index.into(), descriptor_index.into())?,
            ),
            RawEntry::MethodType { descriptor_index } => {
                Entry::MethodType(self.intern_method_descriptor(descriptor_index.into())?)
            }
            RawEntry::Module { name_index } => Entry::Module(ModuleRef {
                name: self.get_utf8(name_index.into())?.to_owned(),
            }),
            RawEntry::Package { name_index } => Entry::Package(
                PackageName::try_from(self.get_utf8(name_index.into())?)
                    .context("Invalid package name")?,
            ),
            RawEntry::FieldRef {
                class_index,
                name_and_type_index,
            } => {
                let owner = self.reference_type_id(class_index.into())?;
                let name_and_type = self.field_name_and_type_id(name_and_type_index.into())?;
                Entry::FieldRef(FieldRefData {
                    owner,
                    name_and_type,
                })
            }
            RawEntry::MethodRef {
                class_index,
                name_and_type_index,
            }
            | RawEntry::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => {
                let interface = matches!(raw, RawEntry::InterfaceMethodRef { .. });
                let owner = self.reference_type_id(class_index.into())?;
                let name_and_type = self.method_name_and_type_id(name_and_type_index.into())?;
                let value = MethodRefData {
                    owner,
                    name_and_type,
                };
                if interface {
                    Entry::InterfaceMethodRef(value)
                } else {
                    Entry::MethodRef(value)
                }
            }
            RawEntry::Dynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                let name_and_type = self.field_name_and_type_id(name_and_type_index.into())?;
                Entry::Dynamic {
                    bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
                    name_and_type,
                }
            }
            RawEntry::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                let name_and_type = self.method_name_and_type_id(name_and_type_index.into())?;
                Entry::InvokeDynamic {
                    bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
                    name_and_type,
                }
            }
            RawEntry::MethodHandle {
                reference_kind,
                reference_index,
            } => Entry::MethodHandle(
                self.resolve_method_handle(reference_kind, reference_index.into())?,
            ),
        })
    }

    fn append_utf8(&mut self, value: &JavaStr) -> Range<usize> {
        let start = self.utf8_bytes.len();
        self.utf8_bytes.extend_from_slice(value.as_bytes());
        start..self.utf8_bytes.len()
    }

    fn resolve_class(&self, name_index: u16) -> Result<Entry, ParseError> {
        let name = self.get_utf8(name_index)?;
        let reference_type = ReferenceType::try_from(name).context("Invalid type reference")?;
        let is_binary_name = !(name.as_bytes().starts_with(b"[")
            || name.as_bytes().starts_with(b"L") && name.as_bytes().ends_with(b";"));
        Ok(Entry::Class {
            reference_type,
            is_binary_name,
        })
    }

    fn resolve_name_and_type(
        &mut self,
        name_index: u16,
        descriptor_index: u16,
    ) -> Result<NameAndType, ParseError> {
        let name = self.utf8_id(name_index)?;
        let descriptor = self.get_utf8(descriptor_index)?;
        Ok(if descriptor.as_bytes().starts_with(b"(") {
            NameAndType::Method(name, self.intern_method_descriptor(descriptor_index)?)
        } else {
            NameAndType::Field(name, self.intern_field_type(descriptor_index)?)
        })
    }

    fn resolve_method_handle(
        &self,
        reference_kind: u8,
        reference_index: u16,
    ) -> Result<MethodHandleData, ParseError> {
        Ok(match reference_kind {
            1 => MethodHandleData::GetField(self.field_ref_id(reference_index)?),
            2 => MethodHandleData::GetStatic(self.field_ref_id(reference_index)?),
            3 => MethodHandleData::PutField(self.field_ref_id(reference_index)?),
            4 => MethodHandleData::PutStatic(self.field_ref_id(reference_index)?),
            5 => MethodHandleData::InvokeVirtual(self.method_ref_id(reference_index)?),
            6 => MethodHandleData::InvokeStatic(self.method_or_interface_ref_id(reference_index)?),
            7 => MethodHandleData::InvokeSpecial(self.method_or_interface_ref_id(reference_index)?),
            8 => MethodHandleData::NewInvokeSpecial(self.method_ref_id(reference_index)?),
            9 => MethodHandleData::InvokeInterface(self.interface_method_ref_id(reference_index)?),
            _ => {
                return Err(ParseError::malform(
                    "Invalid reference kind in method handle",
                ));
            }
        })
    }
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

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
