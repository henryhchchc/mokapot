//! Module for the APIs for the class in JVM.
use std::fmt::Display;

use bitflags::bitflags;

use crate::types::{
    field_type::FieldType,
    signitures::{ClassSignature, FieldSignature},
};

use super::{
    annotation::{Annotation, TypeAnnotation},
    field::{ConstantValue, Field, FieldReference},
    method::{Method, MethodDescriptor, MethodReference},
    module::{Module, PackageReference},
    parsing::Error,
};

/// A JVM class
/// See the [JVM Specification §4](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html) for more information.
#[derive(Debug, Clone)]
pub struct Class {
    /// The version of the class file.
    pub version: ClassVersion,
    /// The access modifiers of the class.
    pub access_flags: ClassAccessFlags,
    /// The binary name of the class (e.g., `org/mokapot/jvm/Class`).
    pub binary_name: String,
    /// A reference to the superclass of the class.
    /// The class `java/lang/Object` has no superclass, so this field is `None` for that class.
    pub super_class: Option<ClassReference>,
    /// The interfaces implemented by the class.
    pub interfaces: Vec<ClassReference>,
    /// The fields declared the class.
    pub fields: Vec<Field>,
    /// The methods declared in the class.
    pub methods: Vec<Method>,
    /// The path to the source file of the class.
    pub source_file: Option<String>,
    /// The inner classes.
    pub inner_classes: Vec<InnerClassInfo>,
    /// The outer class and method of the class.
    pub enclosing_method: Option<EnclosingMethod>,
    /// The source debug extension.
    pub source_debug_extension: Option<SourceDebugExtension>,
    /// The runtime visible annotations.
    pub runtime_visible_annotations: Vec<Annotation>,
    /// The runtime invisible annotations.
    pub runtime_invisible_annotations: Vec<Annotation>,
    /// The runtime visible type annotations.
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    /// The runtime invisible type annotations.
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    /// The bootstrap methods of the class, which are used to generate dynamic callsites.
    pub bootstrap_methods: Vec<BootstrapMethod>,
    /// The infomation of the module if the class is `module-info`.
    pub module: Option<Module>,
    /// The packages of the module.
    pub module_packages: Vec<PackageReference>,
    /// The main class of the module.
    pub module_main_class: Option<ClassReference>,
    /// The nearest outer class of the class.
    pub nest_host: Option<ClassReference>,
    /// The nested classes of the class.
    pub nest_members: Vec<ClassReference>,
    /// The permitted subclasses of the class if the class is `sealed`.
    pub permitted_subclasses: Vec<ClassReference>,
    /// Indicates whether the class is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates whether the class is deprecated.
    pub is_deprecated: bool,
    /// The generic signature of the class.
    pub signature: Option<ClassSignature>,
    /// The record components of the class if the class is `record`.
    pub record: Option<Vec<RecordComponent>>,
    /// Unrecognized JVM attributes.
    pub free_attributes: Vec<(String, Vec<u8>)>,
}

impl Class {
    /// Parses a class file from the given reader.
    /// # Errors
    /// - [`ReadFail`](crate::jvm::parsing::Error::ReadFail) If the reader fails to read.
    /// Other errors may be returned if the class file is malformed.
    /// See [`Error`] for more information.
    pub fn from_reader<R>(reader: R) -> Result<Class, Error>
    where
        R: std::io::Read,
    {
        let mut reader = reader;
        Class::parse(&mut reader)
    }

    /// Gets a method of the class by its name and descriptor.
    #[must_use]
    pub fn get_method(&self, name: &str, descriptor: &MethodDescriptor) -> Option<&Method> {
        self.methods
            .iter()
            .find(|m| m.name == name && &m.descriptor == descriptor)
    }
}

/// The maximum supported major version of a class file.
pub const MAX_MAJOR_VERSION: u16 = 65;

/// The version of a class file.
#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
#[non_exhaustive]
pub enum ClassVersion {
    /// JDK 1.1
    Jdk1_1(u16),
    /// JDK 1.2
    Jdk1_2,
    /// JDK 1.3
    Jdk1_3,
    /// JDK 1.4
    Jdk1_4,
    /// JDK 5
    Jdk5,
    /// JDK 6
    Jdk6,
    /// JDK 7
    Jdk7,
    /// JDK 8
    Jdk8,
    /// JDK 9
    Jdk9,
    /// JDK 10
    Jdk10,
    /// JDK 11
    Jdk11,
    /// JDK 12
    Jdk12(bool),
    /// JDK 13
    Jdk13(bool),
    /// JDK 14
    Jdk14(bool),
    /// JDK 15
    Jdk15(bool),
    /// JDK 16
    Jdk16(bool),
    /// JDK 17
    Jdk17(bool),
    /// JDK 18
    Jdk18(bool),
    /// JDK 19
    Jdk19(bool),
    /// JDK 20
    Jdk20(bool),
    /// JDK 21
    Jdk21(bool),
}
impl ClassVersion {
    pub(crate) const fn from_versions(major: u16, minor: u16) -> Result<Self, Error> {
        match (major, minor) {
            (45, minor) => Ok(Self::Jdk1_1(minor)),
            (46, 0x0000) => Ok(Self::Jdk1_2),
            (47, 0x0000) => Ok(Self::Jdk1_3),
            (48, 0x0000) => Ok(Self::Jdk1_4),
            (49, 0x0000) => Ok(Self::Jdk5),
            (50, 0x0000) => Ok(Self::Jdk6),
            (51, 0x0000) => Ok(Self::Jdk7),
            (52, 0x0000) => Ok(Self::Jdk8),
            (53, 0x0000) => Ok(Self::Jdk9),
            (54, 0x0000) => Ok(Self::Jdk10),
            (55, 0x0000) => Ok(Self::Jdk11),
            (56, 0x0000) => Ok(Self::Jdk12(false)),
            (56, 0xFFFF) => Ok(Self::Jdk12(true)),
            (57, 0x0000) => Ok(Self::Jdk13(false)),
            (57, 0xFFFF) => Ok(Self::Jdk13(true)),
            (58, 0x0000) => Ok(Self::Jdk14(false)),
            (58, 0xFFFF) => Ok(Self::Jdk14(true)),
            (59, 0x0000) => Ok(Self::Jdk15(false)),
            (59, 0xFFFF) => Ok(Self::Jdk15(true)),
            (60, 0x0000) => Ok(Self::Jdk16(false)),
            (60, 0xFFFF) => Ok(Self::Jdk16(true)),
            (61, 0x0000) => Ok(Self::Jdk17(false)),
            (61, 0xFFFF) => Ok(Self::Jdk17(true)),
            (62, 0x0000) => Ok(Self::Jdk18(false)),
            (62, 0xFFFF) => Ok(Self::Jdk18(true)),
            (63, 0x0000) => Ok(Self::Jdk19(false)),
            (63, 0xFFFF) => Ok(Self::Jdk19(true)),
            (64, 0x0000) => Ok(Self::Jdk20(false)),
            (64, 0xFFFF) => Ok(Self::Jdk20(true)),
            (65, 0x0000) => Ok(Self::Jdk21(false)),
            (65, 0xFFFF) => Ok(Self::Jdk21(true)),
            (major, _) if major > MAX_MAJOR_VERSION => {
                Err(Error::MalformedClassFile("Unsupportted class version"))
            }
            _ => Err(Error::MalformedClassFile("Invalid class version")),
        }
    }

    /// Returns `true` if this class file is compiled with `--enable-preview`.
    #[must_use]
    pub const fn is_preview_enabled(&self) -> bool {
        matches!(
            self,
            Self::Jdk12(true)
                | Self::Jdk13(true)
                | Self::Jdk14(true)
                | Self::Jdk15(true)
                | Self::Jdk16(true)
                | Self::Jdk17(true)
                | Self::Jdk18(true)
                | Self::Jdk19(true)
                | Self::Jdk20(true)
                | Self::Jdk21(true)
        )
    }

    /// Returns the major version of the class file.
    #[must_use]
    pub const fn major(&self) -> u16 {
        match self {
            Self::Jdk1_1(_) => 45,
            Self::Jdk1_2 => 46,
            Self::Jdk1_3 => 47,
            Self::Jdk1_4 => 48,
            Self::Jdk5 => 49,
            Self::Jdk6 => 50,
            Self::Jdk7 => 51,
            Self::Jdk8 => 52,
            Self::Jdk9 => 53,
            Self::Jdk10 => 54,
            Self::Jdk11 => 55,
            Self::Jdk12(_) => 56,
            Self::Jdk13(_) => 57,
            Self::Jdk14(_) => 58,
            Self::Jdk15(_) => 59,
            Self::Jdk16(_) => 60,
            Self::Jdk17(_) => 61,
            Self::Jdk18(_) => 62,
            Self::Jdk19(_) => 63,
            Self::Jdk20(_) => 64,
            Self::Jdk21(_) => 65,
        }
    }

    /// Returns the minor version of the class file.
    #[must_use]
    pub const fn minor(&self) -> u16 {
        #[allow(clippy::enum_glob_use)]
        use ClassVersion::*;
        if let Jdk1_1(minor) = self {
            *minor
        } else if let Jdk1_2 | Jdk1_3 | Jdk1_4 | Jdk5 | Jdk6 | Jdk7 | Jdk8 | Jdk9 | Jdk10 | Jdk11
        | Jdk12(false) | Jdk13(false) | Jdk14(false) | Jdk15(false) | Jdk16(false)
        | Jdk17(false) | Jdk18(false) | Jdk19(false) | Jdk20(false) | Jdk21(false) = self
        {
            0
        } else {
            65535
        }
    }
}

/// A reference to a class in the binary format.
#[derive(Debug, PartialEq, Eq, Clone, Hash)]
pub struct ClassReference {
    /// The binary name of the class.
    pub binary_name: String,
}

impl ClassReference {
    /// Creates a new class reference.
    pub fn new<S: Into<String>>(binary_name: S) -> Self {
        ClassReference {
            binary_name: binary_name.into(),
        }
    }
}

impl Display for ClassReference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.binary_name)
    }
}

/// The information of an inner class.
#[derive(Debug, Clone)]
pub struct InnerClassInfo {
    /// The inner class.
    pub inner_class: ClassReference,
    /// The outer class.
    pub outer_class: Option<ClassReference>,
    /// The name of the inner class.
    pub inner_name: Option<String>,
    /// The access flags of the inner class.
    pub inner_class_access_flags: NestedClassAccessFlags,
}

/// The information of an enclosing method of a [`Class`].
#[derive(Debug, Clone)]
pub struct EnclosingMethod {
    /// The class being enclosed.
    pub class: ClassReference,
    /// The name and descriptor of the enclosing method.
    pub method_name_and_desc: Option<(String, MethodDescriptor)>,
}

/// The information of a bootstrap method.
#[derive(Debug, Clone)]
pub struct BootstrapMethod {
    /// The method handle of the bootstrap method.
    pub method: MethodHandle,
    /// The argument that are passed to the bootstrap method.
    pub arguments: Vec<ConstantValue>,
}

/// The source debug extension.
/// See the [JVM Specification §4.7.11](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.11) for more information.
#[derive(Debug, Clone)]
pub struct SourceDebugExtension(Vec<u8>);

impl SourceDebugExtension {
    /// Creates a new source debug extension.
    #[must_use]
    pub fn new(bytes: Vec<u8>) -> Self {
        SourceDebugExtension(bytes)
    }

    /// Gets the bytes of the source debug extension.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.0
    }
}

/// A method handle.
/// See the [JVM Specification §4.4.8](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.4.8) for more information.
#[derive(Debug, PartialEq, Clone)]
pub enum MethodHandle {
    /// Get an instance field.
    RefGetField(FieldReference),
    /// Get a static field.
    RefGetStatic(FieldReference),
    /// Writes to an instance field.
    RefPutField(FieldReference),
    /// Writes to a static field.
    RefPutStatic(FieldReference),
    /// Invoke an instance method.
    RefInvokeVirtual(MethodReference),
    /// Invoke a static method.
    RefInvokeStatic(MethodReference),
    /// Invoke a special method (e.g., a constructor).
    RefInvokeSpecial(MethodReference),
    /// The new version a special method (e.g., a constructor).
    RefNewInvokeSpecial(MethodReference),
    /// Invoke an interface method.
    RefInvokeInterface(MethodReference),
}

/// The record components of a [`Class`] that represents a `record`.
#[derive(Debug, Clone)]
pub struct RecordComponent {
    /// The name of the component.
    pub name: String,
    /// The type of the component.
    pub component_type: FieldType,
    /// The generic signature of the component.
    pub signature: Option<FieldSignature>,
    /// The runtime visible annotations.
    pub runtime_visible_annotations: Vec<Annotation>,
    /// The runtime invisible annotations.
    pub runtime_invisible_annotations: Vec<Annotation>,
    /// The runtime visible type annotations.
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    /// The runtime invisible type annotations.
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    /// Unrecognized JVM attributes.
    pub free_attributes: Vec<(String, Vec<u8>)>,
}

bitflags! {
    /// The access flags of a [`Class`].
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ClassAccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `final`; no subclasses allowed.
        const FINAL = 0x0010;
        /// Treat superclass methods specially when invoked by the invokespecial instruction.
        const SUPER = 0x0020;
        /// Is an interface, not a class.
        const INTERFACE = 0x0200;
        /// Declared `abstract`; must not be instantiated.
        const ABSTRACT = 0x0400;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as an annotation interface.
        const ANNOTATION = 0x2000;
        /// Declared as an enum class.
        const ENUM = 0x4000;
        /// Is a module, not a class or interface.
        const MODULE = 0x8000;
    }
}

bitflags! {
    /// The access flags of a nested class.
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct NestedClassAccessFlags: u16 {
        /// Marked or implicitly `public` in source.
        const PUBLIC = 0x0001;
        /// Marked `private` in source.
        const PRIVATE = 0x0002;
        /// Marked `protected` in source.
        const PROTECTED = 0x0004;
        /// Marked or implicitly `static` in source.
        const STATIC = 0x0008;
        /// Marked `final` in source.
        const FINAL = 0x0010;
        /// Was an `interface` in source.
        const INTERFACE = 0x0200;
        /// Marked or implicitly `abstract` in source.
        const ABSTRACT = 0x0400;
        /// Declared `synthetic`; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as an annotation interface.
        const ANNOTATION = 0x2000;
        /// Declared as an enum class.
        const ENUM = 0x4000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tests::empty_class_with_version;

    #[test]
    fn class_major_version() {
        for major_ver in 45..=65 {
            let bytes = empty_class_with_version(major_ver, 0);
            let class = Class::from_reader(&bytes[..]).expect("Cannot parse class version");
            assert_eq!(major_ver, class.version.major());
        }
    }

    #[test]
    fn class_45_minor_version() {
        for minor_ver in 0..=65535 {
            let bytes = empty_class_with_version(45, minor_ver);
            let class = Class::from_reader(&bytes[..]).expect("Cannot parse class version");
            assert_eq!(minor_ver, class.version.minor());
        }
    }

    #[test]
    fn preview_feature() {
        for major_ver in 56..=65 {
            let bytes = empty_class_with_version(major_ver, 65535);
            let class = Class::from_reader(&bytes[..]).expect("Cannot parse class version");
            assert!(class.version.is_preview_enabled());
        }
    }
}
