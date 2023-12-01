use bitflags::bitflags;

use super::{
    annotation::{Annotation, TypeAnnotation},
    field::{ConstantValue, Field},
    method::Method,
    module::Module,
    references::{ClassReference, FieldReference, MethodReference, PackageReference},
    MethodDescriptor,
};

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
    pub record: Vec<RecordComponent>,
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

#[derive(Debug)]
pub struct InnerClassInfo {
    pub inner_class: ClassReference,
    pub outer_class: Option<ClassReference>,
    pub inner_name: Option<String>,
    pub inner_class_access_flags: u16,
}

#[derive(Debug)]
pub struct EnclosingMethod {
    pub class: ClassReference,
    pub method_name_and_desc: Option<(String, String)>,
}

#[derive(Debug)]
pub struct BootstrapMethod {
    pub method: Handle,
    pub arguments: Vec<ConstantValue>,
}

#[derive(Debug, PartialEq, Clone)]
pub enum Handle {
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
