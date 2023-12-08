use std::str::FromStr;

use super::reader_utils::{read_byte_chunk, ClassReader};
use crate::{
    jvm::ClassFileParsingError,
    jvm::{
        class::{ClassReference, MethodHandle},
        field::{ConstantValue, FieldReference, JavaString},
        method::{MethodDescriptor, MethodReference},
        module::{ModuleReference, PackageReference},
        ClassFileParsingResult,
    },
    types::field_type::{FieldType, TypeReference},
};

/// A JVM constant pool.
/// See the [JVM Specification §4.4](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4) for more information.
#[derive(Debug)]
pub struct ConstantPool {
    entries: Vec<Option<ConstantPoolEntry>>,
}

impl ConstantPool {
    pub(super) fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let constant_pool_count = reader.read_value()?;
        let entries = ConstantPoolEntry::parse_multiple(reader, constant_pool_count)?;

        Ok(Self { entries })
    }

    /// Gets the constant pool entry at the given index.
    pub fn get_entry(&self, index: u16) -> Option<&ConstantPoolEntry> {
        self.entries.get(index as usize).and_then(|it| it.as_ref())
    }

    pub(crate) fn get_entry_internal(
        &self,
        index: u16,
    ) -> ClassFileParsingResult<&ConstantPoolEntry> {
        let Some(entry) = self.get_entry(index) else {
            return Err(ClassFileParsingError::BadConstantPoolIndex(index));
        };
        Ok(entry)
    }

    pub(crate) fn get_str(&self, index: u16) -> ClassFileParsingResult<&str> {
        let entry = self.get_entry_internal(index)?;
        match entry {
            ConstantPoolEntry::Utf8(JavaString::ValidUtf8(string)) => Ok(string),
            ConstantPoolEntry::Utf8(JavaString::InvalidUtf8(_)) => {
                Err(ClassFileParsingError::BrokenUTF8)
            }
            _ => Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "Utf8",
                found: entry.constant_kind(),
            }),
        }
    }

    pub(crate) fn get_class_ref(&self, index: u16) -> ClassFileParsingResult<ClassReference> {
        let entry = self.get_entry_internal(index)?;
        let &ConstantPoolEntry::Class { name_index } = entry else {
            return Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "Class",
                found: entry.constant_kind(),
            });
        };
        let name = self.get_str(name_index)?;
        Ok(ClassReference::new(name))
    }

    pub(crate) fn get_constant_value(
        &self,
        value_index: u16,
    ) -> ClassFileParsingResult<ConstantValue> {
        let entry = self.get_entry_internal(value_index)?;
        match entry {
        &ConstantPoolEntry::Integer(it) => Ok(ConstantValue::Integer(it)),
        &ConstantPoolEntry::Long(it) => Ok(ConstantValue::Long(it)),
        &ConstantPoolEntry::Float(it) => Ok(ConstantValue::Float(it)),
        &ConstantPoolEntry::Double(it) => Ok(ConstantValue::Double(it)),
        &ConstantPoolEntry::String { string_index } => {
            if let ConstantPoolEntry::Utf8(java_str) = self.get_entry_internal(string_index)? {
             Ok(ConstantValue::String(java_str.clone()))
            } else {
                Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                    expected: "Utf8",
                    found: entry.constant_kind(),
                })
            }
        }
        &ConstantPoolEntry::MethodType { descriptor_index } => {
            let descriptor_str = self.get_str(descriptor_index)?;
            let descriptor = MethodDescriptor::from_str(descriptor_str)?;
            Ok(ConstantValue::MethodType(descriptor))
        }
        ConstantPoolEntry::Class { .. } => {
            let class = self.get_class_ref(value_index)?;
            Ok(ConstantValue::Class(class))
        }
        ConstantPoolEntry::MethodHandle { .. } => {
            let method_handle = self.get_method_handle(value_index)?;
            Ok(ConstantValue::Handle(method_handle))
        }
        &ConstantPoolEntry::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        } => {
            let (name, descriptor_str) = self.get_name_and_type(name_and_type_index)?;
            let descriptor = FieldType::from_str(descriptor_str)?;
            Ok(ConstantValue::Dynamic(
                bootstrap_method_attr_index,
                name.to_owned(),
                descriptor,
            ))
        }
        unexpected => Err(ClassFileParsingError::MismatchedConstantPoolEntryType{
            expected: "Integer | Long | Float | Double | String | MethodType | Class | MethodHandle | Dynamic",
            found: unexpected.constant_kind(),
        })
    }
    }

    pub(crate) fn get_module_ref(&self, index: u16) -> ClassFileParsingResult<ModuleReference> {
        let entry = self.get_entry_internal(index)?;
        if let &ConstantPoolEntry::Module { name_index } = entry {
            let name = self.get_str(name_index)?.to_owned();
            return Ok(ModuleReference { name });
        }
        Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
            expected: "Module",
            found: entry.constant_kind(),
        })
    }

    pub(crate) fn get_package_ref(&self, index: u16) -> ClassFileParsingResult<PackageReference> {
        let entry = self.get_entry_internal(index)?;
        if let &ConstantPoolEntry::Package { name_index } = entry {
            let name = self.get_str(name_index)?;
            return Ok(PackageReference {
                binary_name: name.to_owned(),
            });
        }
        Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
            expected: "Package",
            found: entry.constant_kind(),
        })
    }

    pub(crate) fn get_field_ref(&self, index: u16) -> ClassFileParsingResult<FieldReference> {
        let entry = self.get_entry_internal(index)?;
        if let &ConstantPoolEntry::FieldRef {
            class_index,
            name_and_type_index,
        } = entry
        {
            let class = self.get_class_ref(class_index)?;
            if let &ConstantPoolEntry::NameAndType {
                name_index,
                descriptor_index,
            } = self.get_entry_internal(name_and_type_index)?
            {
                let name = self.get_str(name_index)?.to_owned();
                let descriptor = self.get_str(descriptor_index)?;
                let field_type = FieldType::from_str(descriptor)?;
                return Ok(FieldReference {
                    class,
                    name,
                    field_type,
                });
            }
        }
        Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
            expected: "Field",
            found: entry.constant_kind(),
        })
    }

    pub(crate) fn get_name_and_type(&self, index: u16) -> ClassFileParsingResult<(&str, &str)> {
        let entry = self.get_entry_internal(index)?;
        if let &ConstantPoolEntry::NameAndType {
            name_index,
            descriptor_index,
        } = entry
        {
            let name = self.get_str(name_index)?;
            let descriptor = self.get_str(descriptor_index)?;
            return Ok((name, descriptor));
        }
        Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
            expected: "NameAndType",
            found: entry.constant_kind(),
        })?
    }

    pub(crate) fn get_method_ref(&self, index: u16) -> ClassFileParsingResult<MethodReference> {
        let entry = self.get_entry_internal(index)?;
        match entry {
            &ConstantPoolEntry::MethodRef {
                class_index,
                name_and_type_index,
            }
            | &ConstantPoolEntry::InterfaceMethodRef {
                class_index,
                name_and_type_index,
            } => {
                let class_or_interface = self.get_class_ref(class_index)?;
                let (name, descriptor_str) = self.get_name_and_type(name_and_type_index)?;
                let name = name.to_owned();
                let descriptor = MethodDescriptor::from_str(descriptor_str)?;
                Ok(MethodReference {
                    owner: class_or_interface,
                    name,
                    descriptor,
                })
            }
            _ => Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "MethodRef | InterfaceMethodRef",
                found: entry.constant_kind(),
            }),
        }
    }

    pub(crate) fn get_method_handle(&self, index: u16) -> ClassFileParsingResult<MethodHandle> {
        use MethodHandle::*;

        let entry = self.get_entry_internal(index)?;
        let &ConstantPoolEntry::MethodHandle {
            reference_kind,
            reference_index: idx,
        } = entry
        else {
            Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "MethodHandle",
                found: entry.constant_kind(),
            })?
        };

        let result = match reference_kind {
            1 => RefGetField(self.get_field_ref(idx)?),
            2 => RefGetStatic(self.get_field_ref(idx)?),
            3 => RefPutField(self.get_field_ref(idx)?),
            4 => RefPutStatic(self.get_field_ref(idx)?),
            5 => RefInvokeVirtual(self.get_method_ref(idx)?),
            6 => RefInvokeStatic(self.get_method_ref(idx)?),
            7 => RefInvokeSpecial(self.get_method_ref(idx)?),
            8 => RefNewInvokeSpecial(self.get_method_ref(idx)?),
            9 => RefInvokeInterface(self.get_method_ref(idx)?),
            _ => Err(ClassFileParsingError::MalformedClassFile(
                "Invalid reference kind in method handle",
            ))?,
        };
        Ok(result)
    }

    pub(crate) fn get_type_ref(&self, index: u16) -> ClassFileParsingResult<TypeReference> {
        let ClassReference { binary_name: name } = self.get_class_ref(index)?;
        let field_type = if !name.starts_with('[') {
            FieldType::Object(ClassReference::new(name))
        } else {
            FieldType::from_str(name.as_str())?
        };
        Ok(TypeReference(field_type.to_owned()))
    }
}

/// An entry in the [`ConstantPool`].
#[derive(Debug, Clone)]
pub enum ConstantPoolEntry {
    /// A UTF-8 string.
    /// See the [JVM Specification §4.4.7](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.7) for more information.
    Utf8(JavaString),
    /// An integer.
    /// See the [JVM Specification §4.4.4](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.4) for more information.
    Integer(i32),
    /// A float.
    /// See the [JVM Specification §4.4.4](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.4) for more information.
    Float(f32),
    /// A long.
    /// See the [JVM Specification §4.4.5](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.5) for more information.
    Long(i64),
    /// A double.
    /// See the [JVM Specification §4.4.5](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.5) for more information.
    Double(f64),
    /// A class.
    /// See the [JVM Specification §4.4.1](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.1) for more information.
    Class {
        /// The index in the constant pool of its binary name.
        name_index: u16,
    },
    /// A string.
    /// See the [JVM Specification §4.4.3](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.3) for more information.
    String {
        /// The index in the constant pool of its UTF-8 value.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        string_index: u16,
    },
    /// A field reference.
    /// See the [JVM Specification §4.4.2](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.2) for more information.
    FieldRef {
        /// The index in the constant pool of the class containing the field.
        /// The entry at that index must be a [`ConstantPoolEntry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the field.
        /// The entry at that index must be a [`ConstantPoolEntry::NameAndType`].
        name_and_type_index: u16,
    },
    /// A method reference.
    /// See the [JVM Specification §4.4.2](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.2) for more information.
    MethodRef {
        /// The index in the constant pool of the class containing the method.
        /// The entry at that index must be a [`ConstantPoolEntry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`ConstantPoolEntry::NameAndType`].
        name_and_type_index: u16,
    },
    /// An interface method reference.
    InterfaceMethodRef {
        /// The index in the constant pool of the interface containing the method.
        /// The entry at that index must be a [`ConstantPoolEntry::Class`].
        class_index: u16,
        /// The index in the constant pool of the name and type of the method.
        /// The entry at that index must be a [`ConstantPoolEntry::NameAndType`].
        name_and_type_index: u16,
    },
    /// A name and type.
    NameAndType {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        name_index: u16,
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        descriptor_index: u16,
    },
    /// A method handle.
    MethodHandle {
        /// The kind of method handle.
        reference_kind: u8,
        /// The index in the constant pool of the method handle.
        /// The entry at that index must be a [`ConstantPoolEntry::MethodRef`], [`ConstantPoolEntry::InterfaceMethodRef`] or [`ConstantPoolEntry::FieldRef`].
        reference_index: u16,
    },
    /// A method type.
    /// See the [JVM Specification §4.4.9](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.9) for more information.
    MethodType {
        /// The index in the constant pool of the UTF-8 string containing the descriptor.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        descriptor_index: u16,
    },
    /// A dynamically computed constant.
    Dynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`ConstantPoolEntry::NameAndType`].
        name_and_type_index: u16,
    },
    /// An invokedynamic instruction.
    /// See the [JVM Specification §4.4.10](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.10) for more information.
    InvokeDynamic {
        /// The index of the bootstrap method in the bootstrap method table.
        bootstrap_method_attr_index: u16,
        /// The index in the constant pool of the name and type of the constant.
        /// The entry at that index must be a [`ConstantPoolEntry::NameAndType`].
        name_and_type_index: u16,
    },
    /// A module.
    /// See the [JVM Specification §4.4.11](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.11) for more information.
    Module {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        name_index: u16,
    },
    /// A package.
    /// See the [JVM Specification §4.4.12](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.12) for more information.
    Package {
        /// The index in the constant pool of the UTF-8 string containing the name.
        /// The entry at that index must be a [`ConstantPoolEntry::Utf8`].
        name_index: u16,
    },
}

impl ConstantPoolEntry {
    fn parse_multiple<R>(reader: &mut R, count: u16) -> ClassFileParsingResult<Vec<Option<Self>>>
    where
        R: std::io::Read,
    {
        let mut counter: u16 = 1;
        let mut result = vec![None; count as usize];
        while counter < count {
            let entry = Self::parse(reader)?;
            let increment = match entry {
                ConstantPoolEntry::Long(_) | ConstantPoolEntry::Double(_) => 2,
                _ => 1,
            };
            result[counter as usize] = Some(entry);
            counter += increment;
        }
        Ok(result)
    }

    fn parse<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let tag = reader.read_value()?;
        match tag {
            1 => Self::parse_utf8(reader),
            3 => Self::parse_integer(reader),
            4 => Self::parse_float(reader),
            5 => Self::parse_long(reader),
            6 => Self::parse_double(reader),
            7 => Self::parse_class(reader),
            8 => Self::parse_string(reader),
            9 => Self::parse_field_ref(reader),
            10 => Self::parse_method_ref(reader),
            11 => Self::parse_interface_method_ref(reader),
            12 => Self::parse_name_and_type(reader),
            15 => Self::parse_method_handle(reader),
            16 => Self::parse_method_type(reader),
            17 => Self::parse_dynamic(reader),
            18 => Self::parse_invoke_dynamic(reader),
            19 => Self::parse_module(reader),
            20 => Self::parse_package(reader),
            it => Err(ClassFileParsingError::UnexpectedConstantPoolTag(it)),
        }
    }

    /// Gets the kind of this constant pool entry.
    pub const fn constant_kind<'a>(&self) -> &'a str {
        match self {
            Self::Utf8(_) => "CONSTANT_Utf8",
            Self::Integer(_) => "CONSTANT_Integer",
            Self::Float(_) => "CONSTANT_Float",
            Self::Long(_) => "CONSTANT_Long",
            Self::Double(_) => "CONSTANT_Double",
            Self::Class { .. } => "CONSTANT_Class",
            Self::String { .. } => "CONSTANT_String",
            Self::FieldRef { .. } => "CONSTANT_Fieldref",
            Self::MethodRef { .. } => "CONSTANT_Methodref",
            Self::InterfaceMethodRef { .. } => "CONSTANT_InterfaceMethodref",
            Self::NameAndType { .. } => "CONSTANT_NameAndType",
            Self::MethodHandle { .. } => "CONSTANT_MethodHandle",
            Self::MethodType { .. } => "CONSTANT_MethodType",
            Self::Dynamic { .. } => "CONSTANT_Dynamic",
            Self::InvokeDynamic { .. } => "CONSTANT_InvokeDynamic",
            Self::Module { .. } => "CONSTANT_Module",
            Self::Package { .. } => "CONSTANT_Package",
        }
    }

    fn parse_utf8<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let length: u16 = reader.read_value()?;
        let cesu8_content = read_byte_chunk(reader, length as usize)?;
        match cesu8::from_java_cesu8(cesu8_content.as_slice()) {
            Ok(result) => Ok(Self::Utf8(JavaString::ValidUtf8(result.into_owned()))),
            Err(_) => Ok(Self::Utf8(JavaString::InvalidUtf8(cesu8_content))),
        }
    }

    fn parse_integer<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Ok(Self::Integer(reader.read_value()?))
    }

    fn parse_float<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Ok(Self::Float(reader.read_value()?))
    }

    fn parse_long<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Ok(Self::Long(reader.read_value()?))
    }

    fn parse_double<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Ok(Self::Double(reader.read_value()?))
    }

    fn parse_class<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Class { name_index })
    }

    fn parse_string<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let string_index = reader.read_value()?;
        Ok(Self::String { string_index })
    }

    fn parse_field_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::FieldRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_method_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::MethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_interface_method_ref<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let class_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::InterfaceMethodRef {
            class_index,
            name_and_type_index,
        })
    }

    fn parse_name_and_type<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        let descriptor_index = reader.read_value()?;
        Ok(Self::NameAndType {
            name_index,
            descriptor_index,
        })
    }

    fn parse_method_handle<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let reference_kind = reader.read_value()?;
        let reference_index = reader.read_value()?;
        Ok(Self::MethodHandle {
            reference_kind,
            reference_index,
        })
    }

    fn parse_method_type<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let descriptor_index = reader.read_value()?;
        Ok(Self::MethodType { descriptor_index })
    }

    fn parse_dynamic<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::Dynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_invoke_dynamic<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let bootstrap_method_attr_index = reader.read_value()?;
        let name_and_type_index = reader.read_value()?;
        Ok(Self::InvokeDynamic {
            bootstrap_method_attr_index,
            name_and_type_index,
        })
    }

    fn parse_module<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Module { name_index })
    }

    fn parse_package<R>(reader: &mut R) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let name_index = reader.read_value()?;
        Ok(Self::Package { name_index })
    }
}
