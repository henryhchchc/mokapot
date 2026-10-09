//! Eagerly resolved JVM constant-pool values.

use std::io::Read;

use crate::{
    jvm::{
        ConstantValue, JavaString,
        bytecode::constant_pool::{RawConstantPool, RawEntry},
        class::MethodHandle,
        constant_pool_storage::{PoolStorage, Slot},
        errors::{ParseError, ParsingErrorContext},
        references::{FieldRef, MethodRef, ModuleRef},
    },
    types::{
        class_name::ClassName, field_type::FieldType, method_descriptor::MethodDescriptor,
        package_name::PackageName, reference_type::ReferenceType,
    },
};

/// An immutable constant pool whose entries have been resolved to typed values.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantPool {
    inner: PoolStorage<Box<[Slot<Entry>]>>,
}

/// A resolved constant-pool entry. Its original tag is retained.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Entry {
    /// A decoded string, including preserved invalid encodings.
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
    String(JavaString),
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
    MethodType(MethodDescriptor),
    /// A dynamic constant, retaining its bootstrap-table index.
    Dynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Constant name.
        name: String,
        /// Constant type.
        field_type: FieldType,
    },
    /// A dynamic call site, retaining its bootstrap-table index.
    InvokeDynamic {
        /// Index into the class's bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// Call-site name.
        name: String,
        /// Call-site descriptor.
        descriptor: MethodDescriptor,
    },
    /// A module reference.
    Module(ModuleRef),
    /// A package name.
    Package(PackageName),
}

impl Eq for Entry {}

/// A resolved field or method name and descriptor.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameAndType {
    /// A field name and type.
    Field(String, FieldType),
    /// A method name and descriptor.
    Method(String, MethodDescriptor),
}

impl Entry {
    fn type_mismatch(&self, expected: &'static str) -> ParseError {
        ParseError::malform(format!(
            "Mismatched constant pool type. Expected: {expected} but got {}.",
            self.constant_kind()
        ))
    }

    /// Returns the original JVM entry tag.
    #[must_use]
    pub const fn tag(&self) -> u8 {
        match self {
            Self::Utf8(_) => 1,
            Self::Integer(_) => 3,
            Self::Float(_) => 4,
            Self::Long(_) => 5,
            Self::Double(_) => 6,
            Self::Class { .. } => 7,
            Self::String(_) => 8,
            Self::FieldRef(_) => 9,
            Self::MethodRef(_) => 10,
            Self::InterfaceMethodRef(_) => 11,
            Self::NameAndType(_) => 12,
            Self::MethodHandle(_) => 15,
            Self::MethodType(_) => 16,
            Self::Dynamic { .. } => 17,
            Self::InvokeDynamic { .. } => 18,
            Self::Module(_) => 19,
            Self::Package(_) => 20,
        }
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
    /// Creates an empty resolved pool.
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: PoolStorage::with_padding(1),
        }
    }

    /// Reads a pool and resolves all of its entries.
    /// # Errors
    /// Returns binary parsing errors or malformed constant-pool references and values.
    pub fn from_reader<R: Read + ?Sized>(
        reader: &mut R,
        constant_pool_count: u16,
    ) -> Result<Self, ParseError> {
        Self::from_raw(RawConstantPool::from_reader(reader, constant_pool_count)?)
    }

    /// Resolves all entries, preserving indices and reserved slots.
    /// # Errors
    /// Returns an error for invalid references, tags, names, descriptors, or handle kinds.
    pub fn from_raw(raw: RawConstantPool) -> Result<Self, ParseError> {
        let count = raw.count();
        let mut pending = raw.inner.into_slots();
        let mut pool = Self {
            inner: PoolStorage::with_padding(count),
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
    pub fn get_entry(&self, index: u16) -> Option<&Entry> {
        self.inner.get_entry(index)
    }

    /// Returns the indexed slot count, including slot zero and reserved slots.
    #[must_use]
    pub const fn count(&self) -> u16 {
        self.inner.count()
    }

    /// Finds the first entry matching the predicate.
    pub fn find<P: Fn(&Entry) -> bool>(&self, predicate: P) -> Option<(u16, &Entry)> {
        self.inner.find(predicate)
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
    ) -> Result<(u16, String, MethodDescriptor), ParseError> {
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

    pub(crate) fn get_str(&self, index: u16) -> Result<&str, ParseError> {
        match self.entry(index)? {
            Entry::Utf8(JavaString::Utf8(value)) => Ok(value),
            Entry::Utf8(JavaString::InvalidUtf8(_)) => Err(ParseError::malform("Broken UTF-8")),
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
    ) -> Result<(String, FieldType), ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Field(name, ty)) => Ok((name.clone(), ty.clone())),
            entry => Err(entry.type_mismatch("NameAndType with a field descriptor")),
        }
    }

    pub(crate) fn get_method_name_and_type(
        &self,
        index: u16,
    ) -> Result<(String, MethodDescriptor), ParseError> {
        match self.entry(index)? {
            Entry::NameAndType(NameAndType::Method(name, descriptor)) => {
                Ok((name.clone(), descriptor.clone()))
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

    pub(crate) fn get_method_handle(&self, index: u16) -> Result<MethodHandle, ParseError> {
        match self.entry(index)? {
            Entry::MethodHandle(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("MethodHandle")),
        }
    }

    pub(crate) fn get_module_ref(&self, index: u16) -> Result<ModuleRef, ParseError> {
        match self.entry(index)? {
            Entry::Module(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Module")),
        }
    }

    pub(crate) fn get_package_name(&self, index: u16) -> Result<PackageName, ParseError> {
        match self.entry(index)? {
            Entry::Package(value) => Ok(value.clone()),
            entry => Err(entry.type_mismatch("Package")),
        }
    }

    pub(crate) fn get_constant_value(&self, index: u16) -> Result<ConstantValue, ParseError> {
        Ok(match self.entry(index)? {
            Entry::Integer(value) => ConstantValue::Integer(*value),
            Entry::Float(value) => ConstantValue::Float(*value),
            Entry::Long(value) => ConstantValue::Long(*value),
            Entry::Double(value) => ConstantValue::Double(*value),
            Entry::String(value) => ConstantValue::String(value.clone()),
            Entry::Class { reference_type, .. } => ConstantValue::Class(reference_type.clone()),
            Entry::MethodType(value) => ConstantValue::MethodType(value.clone()),
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

    fn resolve_entry(&self, raw: RawEntry) -> Result<Entry, ParseError> {
        Ok(match raw {
            RawEntry::Utf8(bytes) => Entry::Utf8(JavaString::from_modified_utf8(bytes.into_vec())),
            RawEntry::Integer(value) => Entry::Integer(value.into()),
            RawEntry::Float(value) => Entry::Float(value.into()),
            RawEntry::Long(value) => Entry::Long(value.into()),
            RawEntry::Double(value) => Entry::Double(value.into()),
            RawEntry::Class { name_index } => self.resolve_class(name_index.into())?,
            RawEntry::String { string_index } => match self.entry(string_index.into())? {
                Entry::Utf8(value) => Entry::String(value.clone()),
                entry => return Err(entry.type_mismatch("Utf8")),
            },
            RawEntry::NameAndType {
                name_index,
                descriptor_index,
            } => Entry::NameAndType(
                self.resolve_name_and_type(name_index.into(), descriptor_index.into())?,
            ),
            RawEntry::MethodType { descriptor_index } => Entry::MethodType(
                self.get_str(descriptor_index.into())?
                    .parse()
                    .context("Invalid method descriptor")?,
            ),
            RawEntry::Module { name_index } => Entry::Module(ModuleRef {
                name: self.get_str(name_index.into())?.to_owned(),
            }),
            RawEntry::Package { name_index } => Entry::Package(
                self.get_str(name_index.into())?
                    .parse()
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
        let name = self.get_str(name_index)?;
        let reference_type = name.parse().context("Invalid type reference")?;
        let is_binary_name =
            !(name.starts_with('[') || name.starts_with('L') && name.ends_with(';'));
        Ok(Entry::Class {
            reference_type,
            is_binary_name,
        })
    }

    fn resolve_name_and_type(
        &self,
        name_index: u16,
        descriptor_index: u16,
    ) -> Result<NameAndType, ParseError> {
        let name = self.get_str(name_index)?.to_owned();
        let descriptor = self.get_str(descriptor_index)?;
        Ok(if descriptor.starts_with('(') {
            NameAndType::Method(
                name,
                descriptor.parse().context("Invalid method descriptor")?,
            )
        } else {
            NameAndType::Field(
                name,
                descriptor.parse().context("Invalid field descriptor")?,
            )
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

impl Default for ConstantPool {
    fn default() -> Self {
        Self::new()
    }
}

impl RawEntry {
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
    use crate::jvm::{
        bytecode::{ToBytecode, constant_pool::RawEntry},
        errors::ParseErrorKind,
    };

    #[test]
    fn class_and_package_names_round_trip_with_distinct_tags() {
        let class_name: ClassName = "a/b/C".parse().unwrap();
        let package_name: PackageName = "a/b/C".parse().unwrap();
        let mut pool = RawConstantPool::new();
        let class_index = pool.put_class_name(&class_name).unwrap();
        let package_index = pool.put_package_name(&package_name).unwrap();

        assert_ne!(class_index, package_index);
        assert_eq!(pool.count(), 4);
        assert_eq!(pool.put_class_name(&class_name).unwrap(), class_index);
        assert_eq!(pool.put_package_name(&package_name).unwrap(), package_index);
        assert!(matches!(
            pool.get_entry(class_index),
            Some(RawEntry::Class { .. })
        ));
        assert!(matches!(
            pool.get_entry(package_index),
            Some(RawEntry::Package { .. })
        ));

        let mut bytes = Vec::new();
        pool.to_writer(&mut bytes).unwrap();
        let (count_bytes, contents) = bytes.split_at(2);
        let count = u16::from_be_bytes(count_bytes.try_into().unwrap());
        let reparsed = ConstantPool::from_reader(&mut &*contents, count).unwrap();
        assert_eq!(reparsed.get_class_name(class_index).unwrap(), class_name);
        assert_eq!(
            reparsed.get_package_name(package_index).unwrap(),
            package_name
        );
        assert_eq!(
            reparsed.get_class_name(package_index).unwrap_err().kind(),
            ParseErrorKind::Malformed,
        );
        assert_eq!(
            reparsed.get_package_name(class_index).unwrap_err().kind(),
            ParseErrorKind::Malformed,
        );
    }

    #[test]
    fn class_name_accessor_rejects_descriptors() {
        for descriptor in ["[I", "[[I", "[La/b/C;", "[[La/b/C;", "La/b/C;"] {
            let reference_type: ReferenceType = descriptor.parse().unwrap();
            let mut pool = RawConstantPool::new();
            let name_index = pool.put_string(descriptor.to_owned()).unwrap();
            let index = pool
                .put_entry(RawEntry::Class {
                    name_index: name_index.into(),
                })
                .unwrap();

            let pool = ConstantPool::from_raw(pool).unwrap();
            assert_eq!(pool.get_type_ref(index).unwrap(), reference_type);
            assert_eq!(
                pool.get_class_name(index).unwrap_err().kind(),
                ParseErrorKind::Malformed,
            );
        }
    }

    #[test]
    fn miri_interface_method_handles_use_interface_method_refs() {
        let method = MethodRef {
            owner: ReferenceType::Class("a/b/I".parse().unwrap()),
            name: "m".to_owned(),
            descriptor: "()V".parse().unwrap(),
        };
        let handle = MethodHandle::RefInvokeInterface(method);
        let mut pool = RawConstantPool::new();

        let handle_index = pool.put_method_handle(handle.clone()).unwrap();

        let RawEntry::MethodHandle {
            reference_kind,
            reference_index,
        } = pool.get_entry(handle_index).unwrap()
        else {
            panic!("expected method handle entry");
        };
        assert_eq!(*reference_kind, 9);
        assert!(matches!(
            pool.get_entry((*reference_index).into()),
            Some(RawEntry::InterfaceMethodRef { .. })
        ));
        let pool = ConstantPool::from_raw(pool).unwrap();
        assert_eq!(pool.get_method_handle(handle_index).unwrap(), handle);
    }

    #[test]
    fn interface_method_handles_reject_method_refs() {
        let method = MethodRef {
            owner: ReferenceType::Class("a/b/I".parse().unwrap()),
            name: "m".to_owned(),
            descriptor: "()V".parse().unwrap(),
        };
        let mut pool = RawConstantPool::new();
        let method_index = pool.put_method_ref(method).unwrap();
        pool.put_entry(RawEntry::MethodHandle {
            reference_kind: 9,
            reference_index: method_index.into(),
        })
        .unwrap();

        assert_eq!(
            ConstantPool::from_raw(pool).unwrap_err().kind(),
            ParseErrorKind::Malformed
        );
    }

    #[test]
    fn forward_references_resolve_typed_values_and_keep_indices() {
        let mut raw = RawConstantPool::new();
        for entry in [
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
            RawEntry::Utf8(b"a/b/C".as_slice().into()),
            RawEntry::Utf8(b"m".as_slice().into()),
            RawEntry::Utf8(b"(I)V".as_slice().into()),
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
            RawEntry::Utf8(b"I".as_slice().into()),
        ] {
            raw.put_entry(entry).unwrap();
        }
        let count = raw.count();
        let pool = ConstantPool::from_raw(raw).unwrap();
        assert_eq!(pool.count(), count);
        assert_eq!(pool.get_entry(0), None);
        assert_eq!(pool.get_entry(9), None);
        let method = pool.get_method_ref(2).unwrap();
        assert_eq!(method.owner, "a/b/C".parse::<ReferenceType>().unwrap());
        assert_eq!(method.descriptor, "(I)V".parse().unwrap());
        assert_eq!(
            pool.get_method_handle(1).unwrap(),
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
            let mut raw = RawConstantPool::new();
            for entry in [
                RawEntry::Utf8(b"I".as_slice().into()),
                RawEntry::Long(42.into()),
                entry,
            ] {
                raw.put_entry(entry).unwrap();
            }
            assert_eq!(
                ConstantPool::from_raw(raw).unwrap_err().kind(),
                ParseErrorKind::Malformed
            );
        }
    }

    #[test]
    fn utf8_conversion_preserves_strings_and_invalid_bytes() {
        for (bytes, expected) in [
            (b"plain".to_vec(), JavaString::Utf8("plain".to_owned())),
            (vec![0xc0, 0x80], JavaString::Utf8("\0".to_owned())),
            (
                vec![0xed, 0xa0, 0xbd, 0xed, 0xb8, 0x80],
                JavaString::Utf8("😀".to_owned()),
            ),
            (vec![0xff], JavaString::InvalidUtf8(vec![0xff])),
        ] {
            let mut raw = RawConstantPool::new();
            let index = raw
                .put_entry(RawEntry::Utf8(bytes.into_boxed_slice()))
                .unwrap();
            let literal = raw
                .put_entry(RawEntry::String {
                    string_index: index.into(),
                })
                .unwrap();
            let pool = ConstantPool::from_raw(raw).unwrap();
            assert_eq!(pool.get_entry(index), Some(&Entry::Utf8(expected.clone())));
            assert_eq!(
                pool.get_constant_value(literal).unwrap(),
                ConstantValue::String(expected)
            );
        }
    }
}
