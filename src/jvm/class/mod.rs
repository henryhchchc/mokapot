//! JVM classes and interfaces

pub mod constant_pool;

use std::borrow::Borrow;

use bitflags::bitflags;

use super::{
    Annotation, Class, ConstantValue, Field, Method,
    annotation::ElementValue,
    bytecode::ParsingError,
    field,
    references::{ClassRef, FieldRef, MethodRef},
};
use crate::{
    macros::see_jvm_spec,
    types::{field_type::FieldType, method_descriptor::MethodDescriptor},
    utils::enum_discriminant,
};

/// A generic type signature for a class.
pub type Signature = String;

impl Class {
    /// Gets a method of the class by its name and descriptor.
    #[must_use]
    pub fn get_method<D>(&self, name: &str, descriptor: D) -> Option<&Method>
    where
        D: Borrow<MethodDescriptor>,
    {
        self.methods
            .iter()
            .find(|m| m.name == name && &m.descriptor == descriptor.borrow())
    }

    /// Gets a field of the class by its name and type.
    #[must_use]
    pub fn get_field<T>(&self, name: &str, field_type: T) -> Option<&Field>
    where
        T: Borrow<FieldType>,
    {
        self.fields
            .iter()
            .find(|f| f.name == name && &f.field_type == field_type.borrow())
    }

    /// Creates a [`ClassRef`] referring to the class.
    #[must_use]
    pub fn make_ref(&self) -> ClassRef {
        ClassRef {
            binary_name: self.binary_name.clone(),
        }
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

impl Annotation {
    /// The default name of the annotation element.
    pub const DEFAULT_ELEMENT_NAME: &'static str = "value";

    /// Gets the value of the annotation element.
    #[must_use]
    pub fn get_element_value(&self, name: &str) -> Option<&ElementValue> {
        self.element_value_pairs
            .iter()
            .find(|(pair_name, _)| pair_name == name)
            .map(|(_, value)| value)
    }

    /// Gets the value of the `value` element of the annotation.
    #[must_use]
    pub fn get_value(&self) -> Option<&ElementValue> {
        self.get_element_value(Self::DEFAULT_ELEMENT_NAME)
    }
}

/// A JVM constant pool.
#[doc = see_jvm_spec!(4, 4)]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstantPool {
    pub(crate) inner: Vec<constant_pool::Slot>,
}

/// The maximum supported major version of a class file.
pub const MAX_MAJOR_VERSION: u16 = 68;

/// The version of a class file.
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
}
impl Version {
    pub(crate) const fn from_versions(major: u16, minor: u16) -> Result<Self, ParsingError> {
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
            (66, 0x0000) => Ok(Self::Jdk22(false)),
            (66, 0xFFFF) => Ok(Self::Jdk22(true)),
            (67, 0x0000) => Ok(Self::Jdk23(false)),
            (67, 0xFFFF) => Ok(Self::Jdk23(true)),
            (68, 0x0000) => Ok(Self::Jdk24(false)),
            (68, 0xFFFF) => Ok(Self::Jdk24(true)),
            (major, _) if major > MAX_MAJOR_VERSION => {
                Err(ParsingError::Other("Unsupportted class version"))
            }
            _ => Err(ParsingError::Other("Invalid class version")),
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
            | Jdk24(enable_preview) => {
                if *enable_preview {
                    u16::MAX
                } else {
                    0
                }
            }
        }
    }
}

/// The information of an inner class.
#[derive(Debug, Clone)]
pub struct InnerClassInfo {
    /// The inner class.
    pub inner_class: ClassRef,
    /// The outer class.
    pub outer_class: Option<ClassRef>,
    /// The name of the inner class.
    pub inner_name: Option<String>,
    /// The access flags of the inner class.
    pub access_flags: NestedClassAccessFlags,
}

/// The information of an enclosing method of a [`Class`].
#[derive(Debug, Clone)]
pub struct EnclosingMethod {
    /// The class being enclosed.
    pub class: ClassRef,
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

/// A method handle.
#[doc = see_jvm_spec!(4, 4, 8)]
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
#[repr(u8)]
pub enum MethodHandle {
    /// Get an instance field.
    RefGetField(FieldRef) = 1,
    /// Get a static field.
    RefGetStatic(FieldRef) = 2,
    /// Writes to an instance field.
    RefPutField(FieldRef) = 3,
    /// Writes to a static field.
    RefPutStatic(FieldRef) = 4,
    /// Invoke an instance method.
    RefInvokeVirtual(MethodRef) = 5,
    /// Invoke a static method.
    RefInvokeStatic(MethodRef) = 6,
    /// Invoke a special method (e.g., a constructor).
    RefInvokeSpecial(MethodRef) = 7,
    /// The new version a special method (e.g., a constructor).
    RefNewInvokeSpecial(MethodRef) = 8,
    /// Invoke an interface method.
    RefInvokeInterface(MethodRef) = 9,
}

impl MethodHandle {
    /// Gets the reference kind of this method handle.
    #[must_use]
    pub const fn reference_kind(&self) -> u8 {
        // SAFETY: Self is marked as repr(u8)
        unsafe { enum_discriminant(self) }
    }
}

/// The record components of a [`Class`] that represents a `record`.
#[derive(Debug, Clone)]
pub struct RecordComponent {
    /// The name of the component.
    pub name: String,
    /// The type of the component.
    pub component_type: FieldType,
    /// The generic signature of the component.
    pub signature: Option<field::Signature>,
    /// The runtime visible annotations.
    pub runtime_visible_annotations: Vec<super::Annotation>,
    /// The runtime invisible annotations.
    pub runtime_invisible_annotations: Vec<super::Annotation>,
    /// The runtime visible type annotations.
    pub runtime_visible_type_annotations: Vec<super::TypeAnnotation>,
    /// The runtime invisible type annotations.
    pub runtime_invisible_type_annotations: Vec<super::TypeAnnotation>,
    /// Unrecognized JVM attributes.
    pub free_attributes: Vec<(String, Vec<u8>)>,
}

bitflags! {
    /// The access flags of a [`Class`].
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
mod tests {
    use proptest::prelude::*;

    use super::*;

    proptest! {

        #[test]
        fn jdk_1_1(minor in any::<u16>()) {
            let class_version = Version::from_versions(45, minor).unwrap();
            assert_eq!(45, class_version.major());
            assert_eq!(minor, class_version.minor());
        }

        #[test]
        fn jdk_1_x(major in 46u16..56) {
            let class_version = Version::from_versions(major, 0).unwrap();
            assert_eq!(major, class_version.major());
            assert!(!class_version.is_preview_enabled());
        }

        #[test]
        fn jdk_1_x_invalid(
            major in 46u16..56,
            minor in 1u16..
        ) {
            let class_version = Version::from_versions(major, minor);
            assert!(class_version.is_err());
        }

        #[test]
        fn newer_class_versions(
            major in (56..=MAX_MAJOR_VERSION),
            minor in prop_oneof![Just(0u16), Just(u16::MAX)]
        ) {
            let class_version = Version::from_versions(major, minor).unwrap();
            assert_eq!(major, class_version.major());
            assert_eq!(class_version.is_preview_enabled(), class_version.minor() == u16::MAX);
        }

        #[test]
        fn too_low_class_version(major in 0u16..45) {
            assert!(Version::from_versions(major, 0).is_err());
        }

        #[test]
        fn too_high_class_version(major in (MAX_MAJOR_VERSION+1)..=u16::MAX) {
            assert!(Version::from_versions(major, 0).is_err());
        }

        #[test]
        fn invalid_class_version(major in 46..=MAX_MAJOR_VERSION, minor in 1..u16::MAX) {
            assert!(Version::from_versions(major, minor).is_err());
        }
    }

    fn arb_access_flag() -> impl Strategy<Value = AccessFlags> {
        prop_oneof![
            Just(AccessFlags::PUBLIC),
            Just(AccessFlags::PRIVATE),
            Just(AccessFlags::FINAL),
            Just(AccessFlags::SUPER),
            Just(AccessFlags::INTERFACE),
            Just(AccessFlags::ABSTRACT),
            Just(AccessFlags::SYNTHETIC),
            Just(AccessFlags::ANNOTATION),
            Just(AccessFlags::ENUM),
            Just(AccessFlags::MODULE),
        ]
    }

    proptest! {

        #[test]
        fn access_flags_bit_no_overlap(
            lhs in arb_access_flag(),
            rhs in arb_access_flag()
        ){
            prop_assume!(lhs != rhs);
            assert_eq!(lhs.bits() & rhs.bits(), 0);
        }
    }

    #[test]
    fn class_is_abstract() {
        let class = Class {
            access_flags: AccessFlags::PUBLIC | AccessFlags::ABSTRACT,
            ..Default::default()
        };
        assert!(class.is_abstract());

        let class = Class {
            access_flags: AccessFlags::PUBLIC,
            ..Default::default()
        };
        assert!(!class.is_abstract());
    }

    #[test]
    fn class_is_interface() {
        let class = Class {
            access_flags: AccessFlags::PUBLIC | AccessFlags::INTERFACE,
            ..Default::default()
        };
        assert!(class.is_interface());

        let class = Class {
            access_flags: AccessFlags::PUBLIC,
            ..Default::default()
        };
        assert!(!class.is_interface());
    }
}
