//! JVM classes and interfaces

pub(crate) mod bootstrap;
mod metadata;
mod method_handle;
pub(crate) use method_handle::MethodHandleData;
pub use method_handle::{MethodHandle, MethodHndl};
pub(crate) mod nesting;
pub(crate) mod record_component;
pub use bootstrap::BootstrapMethod;
pub use nesting::{EnclosingMethod, InnerClassInfo};
pub use record_component::RecordComponent;

use std::borrow::Borrow;

use bitflags::bitflags;

use super::{Field, Method};
use crate::{
    intrinsics::see_jvm_spec,
    types::{field_type::FieldType, method_descriptor::MethodDescriptor},
};

/// A generic type signature for a class.
pub type Signature = super::JavaString;

/// A JVM class.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug, Clone)]
pub struct Class {
    pub(crate) constant_pool: super::ConstantPool,
    /// The version of the class file.
    pub version: Version,
    /// The access modifiers of the class.
    pub access_flags: AccessFlags,
    /// The class or interface name in JVM internal form (e.g., `org/mokapot/jvm/Class`).
    pub(crate) name: super::constant_pool::ClassId,
    /// A reference to the superclass of the class.
    /// The class `java/lang/Object` has no superclass, so this field is `None` for that class.
    #[expect(clippy::struct_field_names, reason = "Uses the JVM superclass term")]
    pub(crate) super_class: Option<super::constant_pool::ClassId>,
    /// The interfaces implemented by the class.
    pub(crate) interfaces: Box<[super::constant_pool::ClassId]>,
    /// The fields declared the class.
    pub(crate) fields: Box<[super::field::FieldData]>,
    /// The methods declared by the class.
    pub(crate) methods: Box<[super::method::MethodData]>,
    /// The path to the source file.
    pub(crate) source_file: Option<super::constant_pool::Utf8Id>,
    /// The inner classes.
    pub(crate) inner_classes: Box<[nesting::InnerClassData]>,
    /// The outer class and method of the class.
    pub(crate) enclosing_method: Option<nesting::EnclosingMethodData>,
    /// The source debug extension.
    pub source_debug_extension: Option<Box<[u8]>>,
    /// The annotations of the class.
    pub(crate) annotations: super::annotation::AnnotationsData<super::annotation::AnnotationData>,
    /// The type annotations of the class.
    pub(crate) type_annotations:
        super::annotation::AnnotationsData<super::annotation::TypeAnnotationData>,
    /// The bootstrap methods of the class, which are used to generate dynamic callsites.
    pub(crate) bootstrap_methods: Box<[bootstrap::BootstrapMethodData]>,
    /// The information of the module if the class is `module-info`.
    pub(crate) module: Option<super::module::ModuleData>,
    /// The packages of the module.
    pub(crate) module_packages: Box<[super::constant_pool::PackageId]>,
    /// The main class of the module.
    #[expect(clippy::struct_field_names, reason = "Matches the JVM attribute name")]
    pub(crate) module_main_class: Option<super::constant_pool::ClassId>,
    /// The nearest outer class of the class.
    pub(crate) nest_host: Option<super::constant_pool::ClassId>,
    /// The nested classes of the class.
    pub(crate) nest_members: Box<[super::constant_pool::ClassId]>,
    /// The permitted subclasses of the class if the class is `sealed`.
    pub(crate) permitted_subclasses: Box<[super::constant_pool::ClassId]>,
    /// Indicates whether the class is synthesized by the compiler.
    pub is_synthetic: bool,
    /// Indicates whether the class is deprecated.
    pub is_deprecated: bool,
    /// The generic signature of the class.
    pub(crate) signature: Option<super::constant_pool::Utf8Id>,
    /// The record components of the class if the class is `record`.
    pub(crate) record: Option<Box<[record_component::RecordComponentData]>>,
    /// JVM attributes that are not specified by the JVM specification.
    pub(crate) other_attributes: Box<[super::attribute::AttributeData]>,
}

impl Class {
    pub(crate) fn reference_type(&self) -> &crate::types::reference_type::ReferenceType {
        self.constant_pool.class_reference_type(self.name)
    }

    /// Iterates over the methods declared by this class.
    pub fn methods(&self) -> impl ExactSizeIterator<Item = Method<'_>> + DoubleEndedIterator {
        self.methods.iter().map(|data| Method { class: self, data })
    }

    /// Gets a method of the class by its name and descriptor.
    #[must_use]
    pub fn get_method<D>(&self, name: &super::JavaStr, descriptor: D) -> Option<Method<'_>>
    where
        D: Borrow<MethodDescriptor>,
    {
        self.methods()
            .find(|m| m.name() == name && m.descriptor() == descriptor.borrow())
    }

    /// Gets a method using a Rust string name.
    #[must_use]
    pub fn get_method_str<D>(&self, name: &str, descriptor: D) -> Option<Method<'_>>
    where
        D: Borrow<MethodDescriptor>,
    {
        self.methods()
            .find(|method| method.name() == name && method.descriptor() == descriptor.borrow())
    }

    /// Iterates over fields declared by this class.
    pub fn fields(&self) -> impl ExactSizeIterator<Item = Field<'_>> + DoubleEndedIterator {
        self.fields.iter().map(|data| Field { class: self, data })
    }
    /// Gets a field by name and type.
    pub fn get_field<T: Borrow<FieldType>>(
        &self,
        name: &super::JavaStr,
        field_type: T,
    ) -> Option<Field<'_>> {
        self.fields()
            .find(|field| field.name() == name && field.field_type() == field_type.borrow())
    }
    /// Gets a field using a Rust string name.
    pub fn get_field_str<T: Borrow<FieldType>>(
        &self,
        name: &str,
        field_type: T,
    ) -> Option<Field<'_>> {
        self.fields()
            .find(|field| field.name() == name && field.field_type() == field_type.borrow())
    }
    /// Returns record components, if the class is a record.
    #[must_use]
    pub fn record(
        &self,
    ) -> Option<impl ExactSizeIterator<Item = RecordComponent<'_>> + DoubleEndedIterator> {
        self.record.as_ref().map(|items| {
            items
                .iter()
                .map(|data| RecordComponent { class: self, data })
        })
    }
    /// Iterates over inner-class information.
    pub fn inner_classes(
        &self,
    ) -> impl ExactSizeIterator<Item = InnerClassInfo<'_>> + DoubleEndedIterator {
        self.inner_classes
            .iter()
            .map(|data| InnerClassInfo { class: self, data })
    }
    /// Returns the enclosing method information, if present.
    #[must_use]
    pub fn enclosing_method(&self) -> Option<EnclosingMethod<'_>> {
        self.enclosing_method
            .as_ref()
            .map(|data| EnclosingMethod { owner: self, data })
    }
    /// Iterates over bootstrap methods.
    pub fn bootstrap_methods(
        &self,
    ) -> impl ExactSizeIterator<Item = BootstrapMethod<'_>> + DoubleEndedIterator {
        self.bootstrap_methods
            .iter()
            .map(|data| BootstrapMethod { class: self, data })
    }
    /// Returns module information, if present.
    #[must_use]
    pub fn module(&self) -> Option<super::Module<'_>> {
        self.module
            .as_ref()
            .map(|data| super::Module { class: self, data })
    }
    /// Returns annotations on this class.
    #[must_use]
    pub const fn annotations(&self) -> super::Annotations<'_> {
        super::Annotations::new(self, &self.annotations)
    }
    /// Returns type annotations on this class.
    #[must_use]
    pub const fn type_annotations(&self) -> super::TypeAnnotations<'_> {
        super::TypeAnnotations::new(self, &self.type_annotations)
    }

    /// Checks if the class is an interface.
    #[must_use]
    pub const fn is_interface(&self) -> bool {
        self.access_flags.contains(AccessFlags::INTERFACE)
    }

    /// Checks if the class is an abstract class.
    #[must_use]
    pub const fn is_abstract(&self) -> bool {
        self.access_flags.contains(AccessFlags::ABSTRACT)
    }
}

/// The version of a class file.
///
/// Versions through JDK 11 contain the minor version; later versions indicate
/// whether preview features are enabled.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
#[non_exhaustive]
pub enum Version {
    /// JDK 1.1
    Jdk1_1(u16),
    /// JDK 1.2
    Jdk1_2(u16),
    /// JDK 1.3
    Jdk1_3(u16),
    /// JDK 1.4
    Jdk1_4(u16),
    /// JDK 5
    Jdk5(u16),
    /// JDK 6
    Jdk6(u16),
    /// JDK 7
    Jdk7(u16),
    /// JDK 8
    Jdk8(u16),
    /// JDK 9
    Jdk9(u16),
    /// JDK 10
    Jdk10(u16),
    /// JDK 11
    Jdk11(u16),
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
    /// JDK 22
    Jdk22(bool),
    /// JDK 23
    Jdk23(bool),
    /// JDK 24
    Jdk24(bool),
    /// JDK 25
    Jdk25(bool),
    /// JDK 26
    Jdk26(bool),
    /// JDK 27
    Jdk27(bool),
}
impl Version {
    /// The latest supported JDK version (non-preview).
    pub const LATEST: Self = Self::Jdk27(false);

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
                | Self::Jdk22(true)
                | Self::Jdk23(true)
                | Self::Jdk24(true)
                | Self::Jdk25(true)
                | Self::Jdk26(true)
                | Self::Jdk27(true)
        )
    }

    /// Returns the major version of the class file.
    #[must_use]
    pub const fn major(&self) -> u16 {
        match self {
            Self::Jdk1_1(_) => 45,
            Self::Jdk1_2(_) => 46,
            Self::Jdk1_3(_) => 47,
            Self::Jdk1_4(_) => 48,
            Self::Jdk5(_) => 49,
            Self::Jdk6(_) => 50,
            Self::Jdk7(_) => 51,
            Self::Jdk8(_) => 52,
            Self::Jdk9(_) => 53,
            Self::Jdk10(_) => 54,
            Self::Jdk11(_) => 55,
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
            Self::Jdk22(_) => 66,
            Self::Jdk23(_) => 67,
            Self::Jdk24(_) => 68,
            Self::Jdk25(_) => 69,
            Self::Jdk26(_) => 70,
            Self::Jdk27(_) => 71,
        }
    }

    /// Returns the minor version of the class file.
    #[must_use]
    pub const fn minor(&self) -> u16 {
        #[allow(clippy::enum_glob_use)]
        use Version::*;
        match self {
            Jdk1_1(minor) | Jdk1_2(minor) | Jdk1_3(minor) | Jdk1_4(minor) | Jdk5(minor)
            | Jdk6(minor) | Jdk7(minor) | Jdk8(minor) | Jdk9(minor) | Jdk10(minor)
            | Jdk11(minor) => *minor,
            Jdk12(enable_preview)
            | Jdk13(enable_preview)
            | Jdk14(enable_preview)
            | Jdk15(enable_preview)
            | Jdk16(enable_preview)
            | Jdk17(enable_preview)
            | Jdk18(enable_preview)
            | Jdk19(enable_preview)
            | Jdk20(enable_preview)
            | Jdk21(enable_preview)
            | Jdk22(enable_preview)
            | Jdk23(enable_preview)
            | Jdk24(enable_preview)
            | Jdk25(enable_preview)
            | Jdk26(enable_preview)
            | Jdk27(enable_preview) => {
                if *enable_preview {
                    u16::MAX
                } else {
                    0
                }
            }
        }
    }
}

bitflags! {
    /// The access flags of a [`Class`].
    #[doc = see_jvm_spec!(4, 1)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct AccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Marked `private` in source.
        /// NOTE: The is not mentioned in the JVM Specification. However it is set in some class
        /// files, event for those in the JDK.
        const PRIVATE = 0x0002;
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
    #[doc = see_jvm_spec!(4, 7, 6)]
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
        /// Treat superclass methods specially when invoked by the invokespecial instruction.
        /// NOTE: This was not mentioned in the JVM Specification,
        /// but it appears in some class files.
        const SUPER = 0x0020;
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
