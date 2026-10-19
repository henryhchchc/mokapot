//! JVM elements, such as classes, methods, fields, and annotations.

use crate::intrinsics::see_jvm_spec;

pub mod annotation;
pub mod attribute;
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
pub(crate) use class_file::constant_pool::ConstantPool;

mod string;
pub use annotation::{
    Annotation, Annotations, ParameterAnnotations, TypeAnnotation, TypeAnnotations,
};
pub use attribute::Attribute;
pub use class_loader::ClassLoader;
pub use constant_value::ConstantValue;
pub use field::Field;
pub use module::Module;
pub use string::{Cesu8Error, FromCesu8Error, JavaStr, JavaStrDecodeError, JavaString};

/// A JVM class.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug, Clone)]
pub struct Class {
    pub(crate) constant_pool: ConstantPool,
    /// The version of the class file.
    pub version: class::Version,
    /// The access modifiers of the class.
    pub access_flags: class::AccessFlags,
    /// The class or interface name in JVM internal form (e.g., `org/mokapot/jvm/Class`).
    pub(crate) name: class_file::constant_pool::ClassId,
    /// A reference to the superclass of the class.
    /// The class `java/lang/Object` has no superclass, so this field is `None` for that class.
    #[expect(clippy::struct_field_names, reason = "Uses the JVM superclass term")]
    pub(crate) super_class: Option<class_file::constant_pool::ClassId>,
    /// The interfaces implemented by the class.
    pub(crate) interfaces: Box<[class_file::constant_pool::ClassId]>,
    /// The fields declared the class.
    pub(crate) fields: Box<[field::FieldData]>,
    /// The methods declared in the class.
    pub(crate) methods: Box<[method::MethodData]>,
    /// The path to the source file of the class.
    pub(crate) source_file: Option<class_file::constant_pool::Utf8Id>,
    /// The inner classes.
    pub(crate) inner_classes: Box<[class::nesting::InnerClassData]>,
    /// The outer class and method of the class.
    pub(crate) enclosing_method: Option<class::nesting::EnclosingMethodData>,
    /// The source debug extension.
    pub source_debug_extension: Option<Box<[u8]>>,
    /// The annotations of the class.
    pub(crate) annotations: annotation::AnnotationsData<annotation::AnnotationData>,
    /// The type annotations of the class.
    pub(crate) type_annotations: annotation::AnnotationsData<annotation::TypeAnnotationData>,
    /// The bootstrap methods of the class, which are used to generate dynamic callsites.
    pub(crate) bootstrap_methods: Box<[class::bootstrap::BootstrapMethodData]>,
    /// The information of the module if the class is `module-info`.
    pub(crate) module: Option<module::ModuleData>,
    /// The packages of the module.
    pub(crate) module_packages: Box<[class_file::constant_pool::PackageId]>,
    /// The main class of the module.
    #[expect(clippy::struct_field_names, reason = "Matches the JVM attribute name")]
    pub(crate) module_main_class: Option<class_file::constant_pool::ClassId>,
    /// The nearest outer class of the class.
    pub(crate) nest_host: Option<class_file::constant_pool::ClassId>,
    /// The nested classes of the class.
    pub(crate) nest_members: Box<[class_file::constant_pool::ClassId]>,
    /// The permitted subclasses of the class if the class is `sealed`.
    pub(crate) permitted_subclasses: Box<[class_file::constant_pool::ClassId]>,
    /// Indicates whether the class is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates whether the class is deprecated.
    pub is_deprecated: bool,
    /// The generic signature of the class.
    pub(crate) signature: Option<class_file::constant_pool::Utf8Id>,
    /// The record components of the class if the class is `record`.
    pub(crate) record: Option<Box<[class::record_component::RecordComponentData]>>,
    /// JVM attributes that are not specified in the JVM specification.
    pub(crate) other_attributes: Box<[attribute::AttributeData]>,
}

/// A method declared by a JVM class.
#[doc = see_jvm_spec!(4, 6)]
#[derive(Clone, Copy)]
pub struct Method<'class> {
    class: &'class Class,
    data: &'class method::MethodData,
}
