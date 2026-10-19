use std::{fmt, num::NonZeroU16, ops::Range};

pub(crate) mod storage;

use crate::{
    jvm::{
        ConstantValueView, JavaStr,
        class::{MethodHandleData, MethodHandleView},
        constant_pool::storage::{PoolStorage, Slot},
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
pub(crate) struct ConstantPool {
    inner: PoolStorage<Entry>,
    utf8_bytes: Vec<u8>,
    method_descriptors: Vec<MethodDescriptor>,
    field_types: Vec<FieldType>,
    return_types: Vec<ReturnType>,
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

macro_rules! define_pool_ids {
    ($($name:ident($inner:ty)),+ $(,)?) => {
        $(
            #[derive(Debug, Clone, Copy, PartialEq, Eq)]
            pub(crate) struct $name($inner);
        )+
    };
}

define_pool_ids! {
    Utf8Id(NonZeroU16),
    MethodDescriptorId(u16),
    ClassId(NonZeroU16),
    ReferenceTypeId(NonZeroU16),
    PackageId(NonZeroU16),
    ModuleId(NonZeroU16),
    MethodHandleId(NonZeroU16),
    MethodNameAndTypeId(NonZeroU16),
    FieldNameAndTypeId(NonZeroU16),
    FieldTypeId(u16),
    ReturnTypeId(u16),
    ConstantValueId(NonZeroU16),
    FieldRefId(NonZeroU16),
    MethodRefId(NonZeroU16),
    InvokeDynamicId(NonZeroU16),
}

/// A resolved constant-pool entry.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Entry {
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
pub(crate) enum NameAndType {
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
    pub(crate) fn field_type(&self, id: FieldTypeId) -> &FieldType {
        &self.field_types[usize::from(id.0)]
    }
    pub(crate) fn return_type(&self, id: ReturnTypeId) -> &ReturnType {
        &self.return_types[usize::from(id.0)]
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
        &self.method_descriptors[usize::from(id.0)]
    }

    #[cfg(test)]
    #[cfg(test)]
    pub(crate) const fn count(&self) -> u16 {
        self.inner.count()
    }

    /// Gets an entry, or `None` for an invalid index or reserved slot.
    #[must_use]
    fn get_entry(&self, index: u16) -> Option<&Entry> {
        self.inner.get_entry(index)
    }

    fn entry(&self, index: u16) -> Result<&Entry, ParseError> {
        self.get_entry(index).context("Invalid constant pool index")
    }

    pub(crate) fn field_name_and_type_id(
        &self,
        index: u16,
    ) -> Result<FieldNameAndTypeId, ParseError> {
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

    fn append_utf8(&mut self, value: &JavaStr) -> Range<usize> {
        let start = self.utf8_bytes.len();
        self.utf8_bytes.extend_from_slice(value.as_bytes());
        start..self.utf8_bytes.len()
    }
}

impl ConstantPool {
    pub(crate) fn with_slots(count: u16, utf8_capacity: usize) -> Self {
        Self {
            inner: PoolStorage::with_padding(count),
            utf8_bytes: Vec::with_capacity(utf8_capacity),
            method_descriptors: Vec::new(),
            field_types: Vec::new(),
            return_types: Vec::new(),
        }
    }

    pub(crate) fn insert_utf8_at(&mut self, index: u16, value: &JavaStr) {
        let range = self.append_utf8(value);
        self.inner.as_mut_slice()[usize::from(index)] = Slot::Entry(Entry::Utf8(range));
    }

    pub(crate) fn insert_resolved_at(
        &mut self,
        index: u16,
        value: Entry,
    ) -> Result<(), ParseError> {
        if matches!(value, Entry::Utf8(_)) {
            return Err(ParseError::malform("Use validated UTF-8 insertion"));
        }
        self.inner.as_mut_slice()[usize::from(index)] = Slot::Entry(value);
        Ok(())
    }

    pub(crate) fn push_method_descriptor(
        &mut self,
        value: MethodDescriptor,
    ) -> Result<MethodDescriptorId, ParseError> {
        let index = u16::try_from(self.method_descriptors.len())
            .map_err(|_| ParseError::malform("Too many cached values"))?;
        self.method_descriptors.push(value);
        Ok(MethodDescriptorId(index))
    }
    pub(crate) fn push_field_type(&mut self, value: FieldType) -> Result<FieldTypeId, ParseError> {
        let index = u16::try_from(self.field_types.len())
            .map_err(|_| ParseError::malform("Too many cached values"))?;
        self.field_types.push(value);
        Ok(FieldTypeId(index))
    }
    pub(crate) fn push_return_type(
        &mut self,
        value: ReturnType,
    ) -> Result<ReturnTypeId, ParseError> {
        let index = u16::try_from(self.return_types.len())
            .map_err(|_| ParseError::malform("Too many cached values"))?;
        self.return_types.push(value);
        Ok(ReturnTypeId(index))
    }
}

#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
