//! JVM classes and interfaces

pub(crate) mod bootstrap;
mod metadata;
mod method_handle;
pub(crate) use method_handle::MethodHandleData;
pub use method_handle::{MethodHandle, MethodHandleView};
pub(crate) mod nesting;
pub(crate) mod record_component;
pub use bootstrap::BootstrapMethod;
pub use nesting::{EnclosingMethod, InnerClassInfo};
pub use record_component::RecordComponent;

use std::borrow::Borrow;

use bitflags::bitflags;

use super::{Class, Field, Method, errors::ParseError};
use crate::{
    intrinsics::see_jvm_spec,
    types::{field_type::FieldType, method_descriptor::MethodDescriptor},
};

/// A generic type signature for a class.
pub type Signature = super::JavaString;

impl Class {
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
        self.get_method(super::JavaString::from(name).as_java_str(), descriptor)
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

const PREVIEW_FEATURES: u16 = 0xFFFF;

/// The version of a class file.
#[doc = see_jvm_spec!(4, 1)]
#[derive(Debug, PartialOrd, PartialEq, Eq, Copy, Clone)]
#[non_exhaustive]
pub enum Version {
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

    pub(crate) fn from_versions(major: u16, minor: u16) -> Result<Self, ParseError> {
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
            (56, PREVIEW_FEATURES) => Ok(Self::Jdk12(true)),
            (57, 0x0000) => Ok(Self::Jdk13(false)),
            (57, PREVIEW_FEATURES) => Ok(Self::Jdk13(true)),
            (58, 0x0000) => Ok(Self::Jdk14(false)),
            (58, PREVIEW_FEATURES) => Ok(Self::Jdk14(true)),
            (59, 0x0000) => Ok(Self::Jdk15(false)),
            (59, PREVIEW_FEATURES) => Ok(Self::Jdk15(true)),
            (60, 0x0000) => Ok(Self::Jdk16(false)),
            (60, PREVIEW_FEATURES) => Ok(Self::Jdk16(true)),
            (61, 0x0000) => Ok(Self::Jdk17(false)),
            (61, PREVIEW_FEATURES) => Ok(Self::Jdk17(true)),
            (62, 0x0000) => Ok(Self::Jdk18(false)),
            (62, PREVIEW_FEATURES) => Ok(Self::Jdk18(true)),
            (63, 0x0000) => Ok(Self::Jdk19(false)),
            (63, PREVIEW_FEATURES) => Ok(Self::Jdk19(true)),
            (64, 0x0000) => Ok(Self::Jdk20(false)),
            (64, PREVIEW_FEATURES) => Ok(Self::Jdk20(true)),
            (65, 0x0000) => Ok(Self::Jdk21(false)),
            (65, PREVIEW_FEATURES) => Ok(Self::Jdk21(true)),
            (66, 0x0000) => Ok(Self::Jdk22(false)),
            (66, PREVIEW_FEATURES) => Ok(Self::Jdk22(true)),
            (67, 0x0000) => Ok(Self::Jdk23(false)),
            (67, PREVIEW_FEATURES) => Ok(Self::Jdk23(true)),
            (68, 0x0000) => Ok(Self::Jdk24(false)),
            (68, PREVIEW_FEATURES) => Ok(Self::Jdk24(true)),
            (69, 0x0000) => Ok(Self::Jdk25(false)),
            (69, PREVIEW_FEATURES) => Ok(Self::Jdk25(true)),
            (70, 0x0000) => Ok(Self::Jdk26(false)),
            (70, PREVIEW_FEATURES) => Ok(Self::Jdk26(true)),
            (71, 0x0000) => Ok(Self::Jdk27(false)),
            (71, PREVIEW_FEATURES) => Ok(Self::Jdk27(true)),
            (major, _) if major > Self::LATEST.major() => {
                Err(ParseError::malform("Unsupported class version"))
            }
            _ => Err(ParseError::malform("Invalid class version")),
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
            Jdk1_1(minor) => *minor,
            Jdk1_2 | Jdk1_3 | Jdk1_4 | Jdk5 | Jdk6 | Jdk7 | Jdk8 | Jdk9 | Jdk10 | Jdk11 => 0,
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

#[cfg(test)]
mod tests;
