use super::{
    annotation::{Annotation, TypeAnnotation},
    field::Field,
    method::Method,
    module::Module,
    references::{ClassReference, FieldReference, MethodReference, PackageReference},
};

#[derive(Debug)]
pub struct Class {
    pub version: ClassVersion,
    pub access_flags: u16,
    pub this_class: ClassReference,
    pub super_class: ClassReference,
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

#[derive(Debug, Copy, Clone)]
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
    pub inner_name: String,
    pub inner_class_access_flags: u16,
}

#[derive(Debug)]
pub struct EnclosingMethod {
    pub class: ClassReference,
    pub method_name_and_desc: Option<(String, String)>,
}

#[derive(Debug)]
pub struct BootstrapArgument {}

#[derive(Debug)]
pub struct BootstrapMethod {
    pub method: MethodHandle,
    pub argument_indeices: Vec<u16>,
}

#[derive(Debug)]
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
