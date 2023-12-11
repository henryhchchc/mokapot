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
    ClassFileParsingResult,
};

/// APIs for the constant pool in JVM.
pub mod constant_pool {
    pub use super::super::parsing::constant_pool::ConstantPool;
    pub use super::super::parsing::constant_pool::Entry as ConstantPoolEntry;
}

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
}

impl Class {
    /// Parses a class file from the given reader.
    /// # Errors
    /// - [`ClassFileParsingError::ReadFail`](crate::jvm::parsing::errors::ClassFileParsingError::ReadFail) If the reader fails to read.
    /// Other errors may be returned if the class file is malformed.
    /// See [`ClassFileParsingError`](crate::jvm::parsing::errors::ClassFileParsingError) for more information.
    pub fn from_reader<R>(reader: R) -> ClassFileParsingResult<Class>
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

/// The version of a class file.
#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
pub struct ClassVersion {
    /// The major version number.
    pub major: u16,
    /// the minor version number.
    pub minor: u16,
}
impl ClassVersion {
    /// Returns `true` if this class file is compiled with `--enable-preview`.
    #[must_use]
    pub fn is_preview_enabled(&self) -> bool {
        self.minor == 65535
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
