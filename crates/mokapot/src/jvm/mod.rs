//! JVM elements, such as classes, methods, fields, and annotations.

use crate::types::package_name::PackageName;

use crate::{
    intrinsics::see_jvm_spec,
    types::{class_name::ClassName, field_type::FieldType, method_descriptor::MethodDescriptor},
};

pub mod annotation;
#[cfg(feature = "bench-internals")]
#[doc(hidden)]
pub mod benchmark_support;
pub mod class;
mod class_file;
pub mod class_loader;
pub mod code;
pub mod errors;
pub mod field;
pub mod method;
pub mod module;
pub mod references;

mod constant_value;
mod string;
pub use class_loader::ClassLoader;
pub use constant_value::ConstantValue;
pub use string::{JStr, JStrDecodeError, JString};

/// A JVM class.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug, Clone)]
pub struct Class {
    /// The version of the class file.
    pub version: class::Version,
    /// The access modifiers of the class.
    pub access_flags: class::AccessFlags,
    /// The class or interface name in JVM internal form (e.g., `org/mokapot/jvm/Class`).
    pub name: ClassName,
    /// A reference to the superclass of the class.
    /// The class `java/lang/Object` has no superclass, so this field is `None` for that class.
    pub super_class: Option<ClassName>,
    /// The interfaces implemented by the class.
    pub interfaces: Box<[ClassName]>,
    /// The fields declared the class.
    pub fields: Box<[Field]>,
    /// The methods declared in the class.
    pub methods: Box<[Method]>,
    /// The path to the source file of the class.
    pub source_file: Option<String>,
    /// The inner classes.
    pub inner_classes: Box<[class::InnerClassInfo]>,
    /// The outer class and method of the class.
    pub enclosing_method: Option<class::EnclosingMethod>,
    /// The source debug extension.
    pub source_debug_extension: Option<Box<[u8]>>,
    /// The annotations of the class.
    pub annotations: Annotations<Annotation>,
    /// The type annotations of the class.
    pub type_annotations: Annotations<TypeAnnotation>,
    /// The bootstrap methods of the class, which are used to generate dynamic callsites.
    pub bootstrap_methods: Box<[class::BootstrapMethod]>,
    /// The information of the module if the class is `module-info`.
    pub module: Option<Module>,
    /// The packages of the module.
    pub module_packages: Box<[PackageName]>,
    /// The main class of the module.
    pub module_main_class: Option<ClassName>,
    /// The nearest outer class of the class.
    pub nest_host: Option<ClassName>,
    /// The nested classes of the class.
    pub nest_members: Box<[ClassName]>,
    /// The permitted subclasses of the class if the class is `sealed`.
    pub permitted_subclasses: Box<[ClassName]>,
    /// Indicates whether the class is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates whether the class is deprecated.
    pub is_deprecated: bool,
    /// The generic signature of the class.
    pub signature: Option<class::Signature>,
    /// The record components of the class if the class is `record`.
    pub record: Option<Box<[class::RecordComponent]>>,
    /// JVM attributes that are not specified in the JVM specification.
    pub other_attributes: Box<[(String, Box<[u8]>)]>,
}

/// Container for runtime-visible and runtime-invisible annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotations<T> {
    /// The runtime-visible annotations.
    pub runtime_visible: Box<[T]>,
    /// The runtime-invisible annotations.
    pub runtime_invisible: Box<[T]>,
}

impl<T> Default for Annotations<T> {
    fn default() -> Self {
        Self {
            runtime_visible: Box::default(),
            runtime_invisible: Box::default(),
        }
    }
}

/// An annotation on a class, field, method, or parameter.
#[doc = see_jvm_spec!(4, 7, 16)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotation {
    /// The type of the annotation.
    pub annotation_type: FieldType,
    /// The names and values of the annotation's fields.
    pub element_value_pairs: Box<[(String, annotation::ElementValue)]>,
}

/// An type annotation on a class, field, method, or parameter.
#[doc = see_jvm_spec!(4, 7, 20)]
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(
    clippy::module_name_repetitions,
    reason = "To be consistent with JVM spec"
)]
pub struct TypeAnnotation {
    /// The type of the annotation.
    pub annotation_type: FieldType,
    /// Denotes which type of declaration this annotation is on.
    #[doc = see_jvm_spec!(4, 7, 20, 1)]
    pub target_info: annotation::TargetInfo,
    /// The path to the annotated type, describing precisely which part of a compound
    /// type is annotated (e.g., which type argument in a generic type).
    #[doc = see_jvm_spec!(4, 7, 20, 2)]
    pub target_path: Box<[annotation::TypePathElement]>,
    /// The names and values of the annotation's fields.
    pub element_value_pairs: Box<[(String, annotation::ElementValue)]>,
}

/// A JVM field.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Debug, Clone)]
pub struct Field {
    /// The access modifiers of the field.
    pub access_flags: field::AccessFlags,
    /// The name of the field.
    pub name: String,
    /// The class containing the field.
    pub owner: ClassName,
    /// The type of the field.
    pub field_type: FieldType,
    /// The constant value of the field, if any.
    pub constant_value: Option<ConstantValue>,
    /// Indicates if the field is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates if the field is deprecated.
    pub is_deprecated: bool,
    /// The generic signature.
    pub signature: Option<field::Signature>,
    /// The annotations.
    pub annotations: Annotations<Annotation>,
    /// The type annotations.
    pub type_annotations: Annotations<TypeAnnotation>,
    /// Unrecognized JVM attributes.
    pub other_attributes: Box<[(String, Box<[u8]>)]>,
}

/// A JVM method.
#[doc = see_jvm_spec!(4, 6)]
#[derive(Debug, Clone)]
pub struct Method {
    /// The access flags of the method.
    pub access_flags: method::AccessFlags,
    /// The name of the method.
    pub name: String,
    /// The descriptor of the method, encoding parameter types and return type.
    pub descriptor: MethodDescriptor,
    /// The class containing the method.
    pub owner: ClassName,
    /// The body of the method if it is not `abstract` or `native`.
    pub body: Option<code::MethodBody>,
    /// The checked exceptions that may be thrown by the method.
    pub exceptions: Box<[ClassName]>,
    /// The annotations.
    pub annotations: Annotations<Annotation>,
    /// The type annotations.
    pub type_annotations: Annotations<TypeAnnotation>,
    /// The annotations on method parameters.
    pub parameter_annotations: Annotations<Box<[Annotation]>>,
    /// The default value of the annotation (only for annotation interface methods).
    pub annotation_default: Option<annotation::ElementValue>,
    /// The parameters of the method, including names and access flags.
    pub parameters: Box<[method::ParameterInfo]>,
    /// Indicates if the method is synthesized by the compiler (not in source).
    pub is_synthetic: bool,
    /// Indicates if the method is deprecated and should no longer be used.
    pub is_deprecated: bool,
    /// The generic signature for methods with type parameters or generic types.
    pub signature: Option<method::Signature>,
    /// Unrecognized JVM attributes.
    pub other_attributes: Box<[(String, Box<[u8]>)]>,
}

/// A module in the [Java Platform Module System (JPMS)](https://openjdk.org/projects/jigsaw/spec/).
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Module {
    /// The name of the module.
    pub name: String,
    /// The flags of the module.
    pub flags: module::Flags,
    /// The version of the module.
    pub version: Option<String>,
    /// A list of the modules that are required by this module.
    pub requires: Box<[module::Require]>,
    /// A list of the modules that are exported by this module.
    pub exports: Box<[module::Export]>,
    /// A list of the modules that are opened by this module.
    pub opens: Box<[module::Open]>,
    /// A list of the classes that are used by this module.
    pub uses: Box<[ClassName]>,
    /// A list of the services that are provided by this module.
    pub provides: Box<[module::Provide]>,
}
