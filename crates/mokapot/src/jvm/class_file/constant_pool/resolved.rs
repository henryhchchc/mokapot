//! Eagerly resolved JVM constant-pool values.

use std::{borrow::Cow, num::NonZeroU16};

use crate::{
    jvm::{
        ConstantValue, JavaStr, JavaString,
        class::MethodHandle,
        class_file::constant_pool::storage::{PoolStorage, Slot},
        class_file::constant_pool::{RawConstantPool, RawEntry},
        errors::{ParseError, ParsingErrorContext},
        references::{FieldRef, MethodRef, ModuleRef},
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
#[derive(Debug, Clone, PartialEq)]
pub struct ConstantPool {
    inner: PoolStorage<Entry>,
    method_descriptors: ValueCache<MethodDescriptor>,
    field_types: ValueCache<FieldType>,
    return_types: ValueCache<ReturnType>,
    constants: ValueCache<ConstantValue>,
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
    #[cfg(test)]
    fn extend_sources(&mut self, count: u16) {
        self.indices.resize_with(usize::from(count), || None);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Utf8Id(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodDescriptorId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ClassId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct PackageId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ModuleId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodHandleId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MethodNameAndTypeId(NonZeroU16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct FieldTypeId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ReturnTypeId(u16);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct ConstantValueId(u16);

/// A resolved constant-pool entry.
#[derive(Debug, Clone, PartialEq)]
enum Entry {
    /// A string in JVM modified UTF-8 encoding.
    Utf8(JavaString),
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
    FieldRef(FieldRef),
    /// A method reference.
    MethodRef(MethodRef),
    /// An interface method reference.
    InterfaceMethodRef(MethodRef),
    /// A field or method name and descriptor.
    NameAndType(NameAndType),
    /// A method handle.
    MethodHandle(MethodHandle),
    /// A method descriptor.
    MethodType(MethodDescriptorId),
    /// A dynamic constant, retaining its bootstrap-table index.
    Dynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Constant name.
        name: JavaString,
        /// Constant type.
        field_type: FieldType,
    },
    /// A dynamic call site, retaining its bootstrap-table index.
    InvokeDynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Call-site name.
        name: JavaString,
        /// Call-site descriptor.
        descriptor: MethodDescriptor,
    },
    /// A module reference.
    Module(ModuleRef),
    /// A package name.
    Package(PackageName),
}

/// A resolved field or method name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameAndType {
    /// A field name and type.
    Field(JavaString, FieldTypeId),
    /// A method name and descriptor.
    Method(JavaString, MethodDescriptorId),
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
    #[cfg(test)]
    fn extend_test_sources(&mut self) {
        let count = self.inner.count();
        self.method_descriptors.extend_sources(count);
        self.field_types.extend_sources(count);
        self.return_types.extend_sources(count);
        self.constants.extend_sources(count);
    }

    #[cfg(test)]
    pub(crate) fn insert_class_name(&mut self, value: ClassName) -> ClassId {
        let index = self.inner.push(Entry::Class {
            reference_type: ReferenceType::Class(value),
            is_binary_name: true,
        });
        self.extend_test_sources();
        ClassId(NonZeroU16::new(index).expect("nonzero test index"))
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
    pub(crate) fn method_handle(&self, id: MethodHandleId) -> &MethodHandle {
        match self.get_entry(id.0.get()).expect("validated pool ID") {
            Entry::MethodHandle(value) => value,
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
                (name.as_java_str(), self.method_descriptor(*descriptor))
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
    pub(crate) fn intern_constant(&mut self, index: u16) -> Result<ConstantValueId, ParseError> {
        if let Some(id) = self.constants.lookup(index) {
            return Ok(ConstantValueId(id));
        }
        let value = self.get_constant_value(index)?;
        self.constants.insert(index, value).map(ConstantValueId)
    }
    pub(crate) fn constant_value(&self, id: ConstantValueId) -> &ConstantValue {
        self.constants.get(id.0)
    }
    #[cfg(test)]
    pub(crate) fn insert_utf8(&mut self, value: JavaString) -> Utf8Id {
        let index = self.inner.push(Entry::Utf8(value));
        self.extend_test_sources();
        Utf8Id(NonZeroU16::new(index).expect("nonzero test index"))
    }

    #[cfg(test)]
    pub(crate) fn insert_method_descriptor(
        &mut self,
        value: MethodDescriptor,
    ) -> MethodDescriptorId {
        let id = u16::try_from(self.method_descriptors.values.len())
            .expect("test descriptors fit in u16");
        self.method_descriptors.values.push(value);
        MethodDescriptorId(id)
    }

    pub(crate) fn get_utf8(&self, index: u16) -> Result<&JavaStr, ParseError> {
        match self.entry(index)? {
            Entry::Utf8(value) => Ok(value.as_java_str()),
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
        self.constants.finish();
        self
    }

    /// Creates an empty resolved pool.
    #[must_use]
    #[cfg(test)]
    pub fn new() -> Self {
        Self {
            inner: PoolStorage::with_padding(1),
            method_descriptors: ValueCache::new(1),
            field_types: ValueCache::new(1),
            return_types: ValueCache::new(1),
            constants: ValueCache::new(1),
        }
    }

    /// Parses a pool and resolves all of its entries.
    /// Bytes following the specified pool are ignored.
    /// # Errors
    /// Returns binary parsing errors or malformed constant-pool references and values.
    #[cfg(test)]
    pub fn from_bytes(bytes: &[u8], constant_pool_count: u16) -> Result<Self, ParseError> {
        Self::from_raw(RawConstantPool::from_bytes(bytes, constant_pool_count)?)
    }

    /// Resolves all entries, preserving indices and reserved slots.
    /// # Errors
    /// Returns an error for invalid references, tags, names, descriptors, or handle kinds.
    pub fn from_raw(raw: RawConstantPool<'_>) -> Result<Self, ParseError> {
        let count = raw.count();
        let mut pending = raw.inner.into_slots();
        let mut pool = Self {
            inner: PoolStorage::with_padding(count),
            method_descriptors: ValueCache::new(count),
            field_types: ValueCache::new(count),
            return_types: ValueCache::new(count),
            constants: ValueCache::new(count),
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

    /// Returns the indexed slot count, including slot zero and reserved slots.
    #[must_use]
    #[cfg(test)]
    pub const fn count(&self) -> u16 {
        self.inner.count()
    }

    fn entry(&self, index: u16) -> Result<&Entry, ParseError> {
        self.get_entry(index).context("Invalid constant pool index")
    }

    pub(crate) fn get_java_string(&self, index: u16) -> Result<JavaString, ParseError> {
        match self.entry(index)? {
            Entry::Utf8(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Utf8")),
        }
    }

    pub(crate) fn get_invoke_dynamic(
        &self,
        index: u16,
    ) -> Result<(u16, JavaString, MethodDescriptor), ParseError> {
        match self.entry(index)? {
            Entry::InvokeDynamic {
                bootstrap_method_attr_index,
                name,
                descriptor,
            } => Ok((
                *bootstrap_method_attr_index,
                name.clone(),
                descriptor.clone(),
            )),
            entry => Err(entry.type_mismatch("InvokeDynamic")),
        }
    }

    pub(crate) fn get_str(&self, index: u16) -> Result<Cow<'_, str>, ParseError> {
        match self.entry(index)? {
            Entry::Utf8(value) => value
                .to_str()
                .map_err(|_| ParseError::malform("Broken UTF-8")),
            entry => Err(entry.type_mismatch("Utf8")),
        }
    }

    pub(crate) fn get_class_name(&self, index: u16) -> Result<ClassName, ParseError> {
        match self.entry(index)? {
            Entry::Class {
                reference_type: ReferenceType::Class(value),
                is_binary_name: true,
            } => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Class with a non-array name")),
        }
    }

    pub(crate) fn get_type_ref(&self, index: u16) -> Result<ReferenceType, ParseError> {
        match self.entry(index)? {
            Entry::Class { reference_type, .. } => Ok(reference_type.clone()),
            entry => Err(entry.type_mismatch("Class")),
        }
    }

    pub(crate) fn get_field_name_and_type(
        &self,
        index: u16,
    ) -> Result<(JavaString, FieldType), ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Field(name, ty)) => {
                Ok((name.clone(), self.field_type(*ty).clone()))
            }
            entry => Err(entry.type_mismatch("NameAndType with a field descriptor")),
        }
    }

    pub(crate) fn get_method_name_and_type(
        &self,
        index: u16,
    ) -> Result<(JavaString, MethodDescriptor), ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Method(name, descriptor)) => {
                Ok((name.clone(), self.method_descriptor(*descriptor).clone()))
            }
            entry => Err(entry.type_mismatch("NameAndType with a method descriptor")),
        }
    }

    pub(crate) fn get_field_ref(&self, index: u16) -> Result<FieldRef, ParseError> {
        match self.entry(index)? {
            Entry::FieldRef(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Fieldref")),
        }
    }

    pub(crate) fn get_method_ref(&self, index: u16) -> Result<MethodRef, ParseError> {
        match self.entry(index)? {
            Entry::MethodRef(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Methodref")),
        }
    }

    pub(crate) fn get_interface_method_ref(&self, index: u16) -> Result<MethodRef, ParseError> {
        match self.entry(index)? {
            Entry::InterfaceMethodRef(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("InterfaceMethodref")),
        }
    }

    pub(crate) fn get_method_or_interface_ref(&self, index: u16) -> Result<MethodRef, ParseError> {
        match self.entry(index)? {
            Entry::MethodRef(value) | Entry::InterfaceMethodRef(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Methodref | InterfaceMethodref")),
        }
    }

    pub(crate) fn get_constant_value(&self, index: u16) -> Result<ConstantValue, ParseError> {
        Ok(match self.entry(index)? {
            Entry::Integer(value) => ConstantValue::Integer(*value),
            Entry::Float(value) => ConstantValue::Float(*value),
            Entry::Long(value) => ConstantValue::Long(*value),
            Entry::Double(value) => ConstantValue::Double(*value),
            Entry::String(value) => ConstantValue::String(self.utf8(*value).to_owned()),
            Entry::Class { reference_type, .. } => ConstantValue::Class(reference_type.clone()),
            Entry::MethodType(value) => {
                ConstantValue::MethodType(self.method_descriptor(*value).clone())
            }
            Entry::MethodHandle(value) => ConstantValue::Handle(value.clone()),
            Entry::Dynamic {
                bootstrap_method_attr_index,
                name,
                field_type,
            } => ConstantValue::Dynamic(
                *bootstrap_method_attr_index,
                name.clone(),
                field_type.clone(),
            ),
            entry => return Err(entry.type_mismatch("Loadable constant")),
        })
    }

    fn resolve_entry(&mut self, raw: RawEntry<'_>) -> Result<Entry, ParseError> {
        Ok(match raw {
            RawEntry::Utf8(bytes) => Entry::Utf8(
                JavaStr::from_cesu8(bytes)
                    .context("Invalid JVM modified UTF-8")?
                    .to_owned(),
            ),
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
                name: self.get_java_string(name_index.into())?,
            }),
            RawEntry::Package { name_index } => Entry::Package(
                PackageName::try_from(self.get_utf8(name_index.into())?)
                    .context("Invalid package name")?,
            ),
            RawEntry::FieldRef {
                class_index,
                name_and_type_index,
            } => {
                let owner = self.get_type_ref(class_index.into())?;
                let (name, field_type) =
                    self.get_field_name_and_type(name_and_type_index.into())?;
                Entry::FieldRef(FieldRef {
                    owner,
                    name,
                    field_type,
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
                let owner = self.get_type_ref(class_index.into())?;
                let (name, descriptor) =
                    self.get_method_name_and_type(name_and_type_index.into())?;
                let value = MethodRef {
                    owner,
                    name,
                    descriptor,
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
                let (name, field_type) =
                    self.get_field_name_and_type(name_and_type_index.into())?;
                Entry::Dynamic {
                    bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
                    name,
                    field_type,
                }
            }
            RawEntry::InvokeDynamic {
                bootstrap_method_attr_index,
                name_and_type_index,
            } => {
                let (name, descriptor) =
                    self.get_method_name_and_type(name_and_type_index.into())?;
                Entry::InvokeDynamic {
                    bootstrap_method_attr_index: bootstrap_method_attr_index.into(),
                    name,
                    descriptor,
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
        let name = self.get_java_string(name_index)?;
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
    ) -> Result<MethodHandle, ParseError> {
        Ok(match reference_kind {
            1 => MethodHandle::RefGetField(self.get_field_ref(reference_index)?),
            2 => MethodHandle::RefGetStatic(self.get_field_ref(reference_index)?),
            3 => MethodHandle::RefPutField(self.get_field_ref(reference_index)?),
            4 => MethodHandle::RefPutStatic(self.get_field_ref(reference_index)?),
            5 => MethodHandle::RefInvokeVirtual(self.get_method_ref(reference_index)?),
            6 => MethodHandle::RefInvokeStatic(self.get_method_or_interface_ref(reference_index)?),
            7 => MethodHandle::RefInvokeSpecial(self.get_method_or_interface_ref(reference_index)?),
            8 => MethodHandle::RefNewInvokeSpecial(self.get_method_ref(reference_index)?),
            9 => MethodHandle::RefInvokeInterface(self.get_interface_method_ref(reference_index)?),
            _ => {
                return Err(ParseError::malform(
                    "Invalid reference kind in method handle",
                ));
            }
        })
    }
}

#[cfg(test)]
impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
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
mod tests {
    use super::*;
    use crate::jvm::{class_file::constant_pool::RawEntry, errors::ParseErrorKind};
    use crate::types::Descriptor;

    #[test]
    fn checked_ids_preserve_values_and_share_parsed_descriptors() {
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(b"()V"),
            RawEntry::Long(0.into()),
            RawEntry::Utf8(&[0xed, 0xa0, 0x80]),
            RawEntry::Utf8(b"I"),
            RawEntry::Utf8(b"V"),
        ]);
        let mut pool = ConstantPool::from_raw(raw).unwrap();
        for index in [0, 2, 3, 7] {
            assert_eq!(
                pool.utf8_id(index).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
            assert_eq!(
                pool.intern_method_descriptor(index).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
        let name = pool.utf8_id(4).unwrap();
        let descriptor = pool.intern_method_descriptor(1).unwrap();
        assert_eq!(pool.intern_method_descriptor(1).unwrap(), descriptor);
        assert_eq!(pool.method_descriptors.values.len(), 1);
        let field_type = pool.intern_field_type(5).unwrap();
        assert_eq!(pool.intern_field_type(5).unwrap(), field_type);
        let return_type = pool.intern_return_type(6).unwrap();
        assert_eq!(pool.intern_return_type(6).unwrap(), return_type);
        let constant = pool.intern_constant(2).unwrap();
        assert_eq!(pool.intern_constant(2).unwrap(), constant);
        for index in [0, 3, 7] {
            for result in [
                pool.intern_field_type(index).map(|_| ()),
                pool.intern_return_type(index).map(|_| ()),
                pool.intern_constant(index).map(|_| ()),
            ] {
                assert_eq!(result.unwrap_err().kind(), ParseErrorKind::Malformed);
            }
        }
        let pool = pool.finish();
        assert_eq!(pool.utf8(name).as_bytes(), &[0xed, 0xa0, 0x80]);
        assert_eq!(pool.method_descriptor(descriptor).descriptor(), "()V");
        assert_eq!(pool.field_type(field_type).descriptor(), "I");
        assert_eq!(pool.return_type(return_type), &ReturnType::Void);
        assert_eq!(pool.constant_value(constant), &ConstantValue::Long(0));
    }

    #[test]
    fn class_and_package_names_parse_with_distinct_tags() {
        // Utf8 "a/b/C", Class #1, Package #1.
        let bytes = [1, 0, 5, b'a', b'/', b'b', b'/', b'C', 7, 0, 1, 20, 0, 1];
        let pool = ConstantPool::from_bytes(bytes.as_slice(), 4).unwrap();
        let class_name: ClassName = "a/b/C".parse().unwrap();
        let package_name: PackageName = "a/b/C".parse().unwrap();
        assert_eq!(pool.count(), 4);
        assert_eq!(pool.get_class_name(2).unwrap(), class_name);
        assert_eq!(
            pool.package_name(pool.package_id(3).unwrap()).clone(),
            package_name
        );
        assert_eq!(
            pool.get_class_name(3).unwrap_err().kind(),
            ParseErrorKind::Malformed,
        );
        assert_eq!(
            pool.package_id(2).unwrap_err().kind(),
            ParseErrorKind::Malformed,
        );
    }

    #[test]
    fn class_name_accessor_rejects_descriptors() {
        for descriptor in ["[I", "[La/b/C;", "La/b/C;"] {
            let reference_type: ReferenceType = descriptor.parse().unwrap();
            let pool = RawConstantPool::from_entries([
                RawEntry::Utf8(descriptor.as_bytes()),
                RawEntry::Class {
                    name_index: 1.into(),
                },
            ]);
            let index = 2;

            let pool = ConstantPool::from_raw(pool).unwrap();
            assert_eq!(pool.get_type_ref(index).unwrap(), reference_type);
            assert_eq!(
                pool.get_class_name(index).unwrap_err().kind(),
                ParseErrorKind::Malformed,
            );
        }
    }

    fn interface_method_handle_bytes(reference_tag: u8) -> Vec<u8> {
        // Utf8 owner, Class #1, Utf8 name, Utf8 descriptor, NameAndType #3:#4,
        // method reference #2:#5, REF_invokeInterface handle #6.
        let bytes = [
            1,
            0,
            5,
            b'a',
            b'/',
            b'b',
            b'/',
            b'I',
            7,
            0,
            1,
            1,
            0,
            1,
            b'm',
            1,
            0,
            3,
            b'(',
            b')',
            b'V',
            12,
            0,
            3,
            0,
            4,
            reference_tag,
            0,
            2,
            0,
            5,
            15,
            9,
            0,
            6,
        ];
        bytes.to_vec()
    }

    #[test]
    fn interface_method_handles_use_interface_method_refs() {
        let method = MethodRef {
            owner: ReferenceType::Class("a/b/I".parse().unwrap()),
            name: "m".into(),
            descriptor: "()V".parse().unwrap(),
        };
        let handle = MethodHandle::RefInvokeInterface(method);
        let bytes = interface_method_handle_bytes(11);
        let pool = ConstantPool::from_bytes(&bytes, 8).unwrap();
        assert_eq!(
            pool.method_handle(pool.method_handle_id(7).unwrap())
                .clone(),
            handle
        );
    }

    #[test]
    fn interface_method_handles_reject_method_refs() {
        let bytes = interface_method_handle_bytes(10);
        assert_eq!(
            ConstantPool::from_bytes(&bytes, 8).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }

    #[test]
    fn forward_references_resolve_typed_values_and_keep_indices() {
        let raw = RawConstantPool::from_entries([
            RawEntry::MethodHandle {
                reference_kind: 5,
                reference_index: 2.into(),
            },
            RawEntry::MethodRef {
                class_index: 3.into(),
                name_and_type_index: 4.into(),
            },
            RawEntry::Class {
                name_index: 5.into(),
            },
            RawEntry::NameAndType {
                name_index: 6.into(),
                descriptor_index: 7.into(),
            },
            RawEntry::Utf8(b"a/b/C".as_slice()),
            RawEntry::Utf8(b"m".as_slice()),
            RawEntry::Utf8(b"(I)V".as_slice()),
            RawEntry::Long(42.into()),
            RawEntry::Dynamic {
                bootstrap_method_attr_index: 3.into(),
                name_and_type_index: 12.into(),
            },
            RawEntry::InvokeDynamic {
                bootstrap_method_attr_index: 4.into(),
                name_and_type_index: 4.into(),
            },
            RawEntry::NameAndType {
                name_index: 6.into(),
                descriptor_index: 13.into(),
            },
            RawEntry::Utf8(b"I".as_slice()),
        ]);
        let count = raw.count();
        let pool = ConstantPool::from_raw(raw).unwrap();
        assert_eq!(pool.count(), count);
        assert_eq!(pool.get_entry(0), None);
        assert_eq!(pool.get_entry(9), None);
        let method = pool.get_method_ref(2).unwrap();
        assert_eq!(method.owner, "a/b/C".parse::<ReferenceType>().unwrap());
        assert_eq!(method.descriptor, "(I)V".parse().unwrap());
        assert_eq!(
            pool.method_handle(pool.method_handle_id(1).unwrap())
                .clone(),
            MethodHandle::RefInvokeVirtual(method)
        );
        assert!(matches!(
            pool.get_entry(10),
            Some(Entry::Dynamic {
                bootstrap_method_attr_index: 3,
                ..
            })
        ));
        assert!(matches!(
            pool.get_entry(11),
            Some(Entry::InvokeDynamic {
                bootstrap_method_attr_index: 4,
                ..
            })
        ));
    }

    #[test]
    fn resolution_rejects_invalid_dependencies_and_descriptors() {
        for entry in [
            RawEntry::Class {
                name_index: 0.into(),
            },
            RawEntry::Class {
                name_index: 3.into(),
            },
            RawEntry::Class {
                name_index: 2.into(),
            },
            RawEntry::Class {
                name_index: u16::MAX.into(),
            },
            RawEntry::MethodType {
                descriptor_index: 1.into(),
            },
            RawEntry::MethodHandle {
                reference_kind: 0,
                reference_index: 1.into(),
            },
        ] {
            let raw = RawConstantPool::from_entries([
                RawEntry::Utf8(b"I".as_slice()),
                RawEntry::Long(42.into()),
                entry,
            ]);
            assert_eq!(
                ConstantPool::from_raw(raw).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
    }

    #[test]
    fn string_entries_preserve_jvm_bytes() {
        let bytes = b"a\xc0\x80\xed\xa0\xbd\xed\xb8\x80\xed\xa0\x80";
        let expected = JavaString::from_cesu8(bytes.to_vec()).unwrap();
        let raw = RawConstantPool::from_entries([
            RawEntry::Utf8(bytes),
            RawEntry::String {
                string_index: 1.into(),
            },
        ]);
        let pool = ConstantPool::from_raw(raw).unwrap();
        assert_eq!(pool.get_java_string(1).unwrap().as_bytes(), bytes);
        assert_eq!(
            pool.get_constant_value(2).unwrap(),
            ConstantValue::String(expected)
        );
    }

    #[test]
    fn malformed_utf8_entries_are_rejected_without_references() {
        let raw = RawConstantPool::from_entries([RawEntry::Utf8(b"\xff")]);
        assert_eq!(
            ConstantPool::from_raw(raw).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }
}
