use std::fmt::Display;

use bitflags::bitflags;

use super::{
    annotation::{Annotation, TypeAnnotation},
    field::{ConstantValue, Field, FieldReference},
    method::{Method, MethodDescriptor, MethodReference},
    module::{Module, PackageReference},
};

pub use super::parsing::errors::ClassFileParsingError;

#[derive(Debug)]
pub struct Class {
    pub version: ClassVersion,
    pub access_flags: ClassAccessFlags,
    pub binary_name: String,
    pub super_class: Option<ClassReference>,
    pub interfaces: Vec<ClassReference>,
    pub fields: Vec<Field>,
    pub methods: Vec<Method>,
    pub source_file: Option<String>,
    pub inner_classes: Vec<InnerClassInfo>,
    pub enclosing_method: Option<EnclosingMethod>,
    pub source_debug_extension: Vec<u8>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
    pub bootstrap_methods: Vec<BootstrapMethod>,
    pub module: Option<Module>,
    pub module_packages: Vec<PackageReference>,
    pub module_main_class: Option<ClassReference>,
    pub nest_host: Option<ClassReference>,
    pub nest_members: Vec<ClassReference>,
    pub permitted_subclasses: Vec<ClassReference>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<String>,
    pub record: Option<Vec<RecordComponent>>,
}

impl Class {
    pub fn get_method(&self, name: &str, descriptor: MethodDescriptor) -> Option<&Method> {
        self.methods
            .iter()
            .find(|m| m.name == name && m.descriptor == descriptor)
    }
}

#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
/// The version of a class file.
pub struct ClassVersion {
    /// The major version number.
    pub major: u16,
    /// the minor version number.
    pub minor: u16,
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

#[derive(Debug)]
pub struct InnerClassInfo {
    pub inner_class: ClassReference,
    pub outer_class: Option<ClassReference>,
    pub inner_name: Option<String>,
    pub inner_class_access_flags: NestedClassAccessFlags,
}

#[derive(Debug)]
pub struct EnclosingMethod {
    pub class: ClassReference,
    pub method_name_and_desc: Option<(String, MethodDescriptor)>,
}

#[derive(Debug)]
pub struct BootstrapMethod {
    pub method: MethodHandle,
    pub arguments: Vec<ConstantValue>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum MethodHandle {
    RefGetField(FieldReference),
    RefGetStatic(FieldReference),
    RefPutField(FieldReference),
    RefPutStatic(FieldReference),
    RefInvokeVirtual(MethodReference),
    RefInvokeStatic(MethodReference),
    RefInvokeSpecial(MethodReference),
    RefNewInvokeSpecial(MethodReference),
    RefInvokeInterface(MethodReference),
}

#[derive(Debug)]
pub struct RecordComponent {
    pub name: String,
    pub descriptor: String,
    pub signature: Option<String>,
    pub runtime_visible_annotations: Vec<Annotation>,
    pub runtime_invisible_annotations: Vec<Annotation>,
    pub runtime_visible_type_annotations: Vec<TypeAnnotation>,
    pub runtime_invisible_type_annotations: Vec<TypeAnnotation>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
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

    #[derive(Debug, PartialEq, Eq)]
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
