//! JVM elements, such as classes, methods, fields, and annotations.

use crate::types::package_name::PackageName;

use derive_more::Display;
use itertools::Itertools;

use crate::{
    intrinsics::see_jvm_spec,
    types::{class_name::ClassName, field_type::FieldType, method_descriptor::MethodDescriptor},
};

pub mod annotation;
pub mod bytecode;
pub mod class;
pub mod class_loader;
pub mod code;
pub mod errors;
pub mod field;
pub mod method;
pub mod module;
pub mod references;

mod constant_pool_storage;
mod constant_value;
pub use class_loader::ClassLoader;
pub use constant_value::ConstantValue;

/// A JVM class
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
    pub interfaces: Vec<ClassName>,
    /// The fields declared the class.
    pub fields: Vec<Field>,
    /// The methods declared in the class.
    pub methods: Vec<Method>,
    /// The path to the source file of the class.
    pub source_file: Option<String>,
    /// The inner classes.
    pub inner_classes: Vec<class::InnerClassInfo>,
    /// The outer class and method of the class.
    pub enclosing_method: Option<class::EnclosingMethod>,
    /// The source debug extension.
    pub source_debug_extension: Option<Vec<u8>>,
    /// The annotations of the class.
    pub annotations: Annotations<Annotation>,
    /// The type annotations of the class.
    pub type_annotations: Annotations<TypeAnnotation>,
    /// The bootstrap methods of the class, which are used to generate dynamic callsites.
    pub bootstrap_methods: Vec<class::BootstrapMethod>,
    /// The information of the module if the class is `module-info`.
    pub module: Option<Module>,
    /// The packages of the module.
    pub module_packages: Vec<PackageName>,
    /// The main class of the module.
    pub module_main_class: Option<ClassName>,
    /// The nearest outer class of the class.
    pub nest_host: Option<ClassName>,
    /// The nested classes of the class.
    pub nest_members: Vec<ClassName>,
    /// The permitted subclasses of the class if the class is `sealed`.
    pub permitted_subclasses: Vec<ClassName>,
    /// Indicates whether the class is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates whether the class is deprecated.
    pub is_deprecated: bool,
    /// The generic signature of the class.
    pub signature: Option<class::Signature>,
    /// The record components of the class if the class is `record`.
    pub record: Option<Vec<class::RecordComponent>>,
    /// JVM attributes that are not specified in the JVM specification.
    pub other_attributes: Vec<(String, Vec<u8>)>,
}

/// Container for runtime-visible and runtime-invisible annotations.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Annotations<T> {
    /// The runtime-visible annotations.
    pub runtime_visible: Vec<T>,
    /// The runtime-invisible annotations.
    pub runtime_invisible: Vec<T>,
}

impl<T> Default for Annotations<T> {
    fn default() -> Self {
        Self {
            runtime_visible: Vec::new(),
            runtime_invisible: Vec::new(),
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
    pub element_value_pairs: Vec<(String, annotation::ElementValue)>,
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
    pub target_path: Vec<annotation::TypePathElement>,
    /// The names and values of the annotation's fields.
    pub element_value_pairs: Vec<(String, annotation::ElementValue)>,
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
    pub other_attributes: Vec<(String, Vec<u8>)>,
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
    pub exceptions: Vec<ClassName>,
    /// The annotations.
    pub annotations: Annotations<Annotation>,
    /// The type annotations.
    pub type_annotations: Annotations<TypeAnnotation>,
    /// The annotations on method parameters.
    pub parameter_annotations: Annotations<Vec<Annotation>>,
    /// The default value of the annotation (only for annotation interface methods).
    pub annotation_default: Option<annotation::ElementValue>,
    /// The parameters of the method, including names and access flags.
    pub parameters: Vec<method::ParameterInfo>,
    /// Indicates if the method is synthesized by the compiler (not in source).
    pub is_synthetic: bool,
    /// Indicates if the method is deprecated and should no longer be used.
    pub is_deprecated: bool,
    /// The generic signature for methods with type parameters or generic types.
    pub signature: Option<method::Signature>,
    /// Unrecognized JVM attributes.
    pub other_attributes: Vec<(String, Vec<u8>)>,
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
    pub requires: Vec<module::Require>,
    /// A list of the modules that are exported by this module.
    pub exports: Vec<module::Export>,
    /// A list of the modules that are opened by this module.
    pub opens: Vec<module::Open>,
    /// A list of the classes that are used by this module.
    pub uses: Vec<ClassName>,
    /// A list of the services that are provided by this module.
    pub provides: Vec<module::Provide>,
}

/// A string in the JVM bytecode.
#[derive(PartialEq, Eq, Debug, PartialOrd, Ord, Clone, Hash, Display)]
#[cfg_attr(test, derive(proptest_derive::Arbitrary))]
pub enum JavaString {
    /// A valid UTF-8 string.
    #[display("String(\"{_0}\")")]
    Utf8(String),
    /// An string that is not valid UTF-8.
    #[display("String({}) // Invalid UTF-8", _0.iter().map(|it| format!("0x{it:02X}")).join(" "))]
    InvalidUtf8(Vec<u8>),
}

impl JavaString {
    pub(crate) fn from_modified_utf8(bytes: Vec<u8>) -> Self {
        match String::from_utf8(bytes) {
            Ok(value) => Self::Utf8(value),
            Err(error) => match cesu8::from_java_cesu8(error.as_bytes()) {
                Ok(value) => Self::Utf8(value.into_owned()),
                Err(_) => Self::InvalidUtf8(error.into_bytes()),
            },
        }
    }

    pub(crate) fn into_modified_utf8(self) -> Vec<u8> {
        match self {
            Self::Utf8(value) => cesu8::to_java_cesu8(&value).into_owned(),
            Self::InvalidUtf8(bytes) => bytes,
        }
    }
}
