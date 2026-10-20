//! JVM methods.

use std::fmt;

use bitflags::bitflags;

use super::{
    Annotations, Class, DebugList, JavaStr, JavaString, annotation,
    annotation::{
        AnnotationData, AnnotationsData, ElementValueData, ParameterAnnotations,
        TypeAnnotationData, TypeAnnotations,
    },
    attribute::{Attribute, AttributeData},
    code,
    constant_pool::{ClassId, MethodDescriptorId, Utf8Id},
    references::MethodRef,
};
use crate::intrinsics::see_jvm_spec;
use crate::types::{class_name::ClassName, method_descriptor::MethodDescriptor};

/// A generic type signature for a method.
pub type Signature = JavaString;

/// A method declared by a JVM class.
#[doc = see_jvm_spec!(4, 6)]
#[derive(Clone, Copy)]
pub struct Method<'class> {
    pub(super) class: &'class Class,
    pub(super) data: &'class MethodData,
}

#[derive(Debug, Clone)]
pub(crate) struct MethodData {
    pub access_flags: AccessFlags,
    pub name: Utf8Id,
    pub descriptor: MethodDescriptorId,
    pub body: Option<code::MethodBodyData>,
    pub exceptions: Box<[ClassId]>,
    pub annotations: AnnotationsData<AnnotationData>,
    pub type_annotations: AnnotationsData<TypeAnnotationData>,
    pub parameter_annotations: AnnotationsData<Box<[AnnotationData]>>,
    pub annotation_default: Option<ElementValueData>,
    pub parameters: Box<[ParameterInfoData]>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<Utf8Id>,
    pub other_attributes: Box<[AttributeData]>,
}

impl<'class> Method<'class> {
    /// The class declaring this method.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.class
    }

    /// The method access flags.
    #[must_use]
    pub const fn access_flags(&self) -> AccessFlags {
        self.data.access_flags
    }

    /// The method name.
    #[must_use]
    pub fn name(&self) -> &'class JavaStr {
        self.class.constant_pool.utf8(self.data.name)
    }

    /// The method descriptor.
    #[must_use]
    pub fn descriptor(&self) -> &'class MethodDescriptor {
        self.class
            .constant_pool
            .method_descriptor(self.data.descriptor)
    }

    /// The method body, if present.
    #[must_use]
    pub fn body(&self) -> Option<code::MethodBody<'class>> {
        self.data
            .body
            .as_ref()
            .map(|data| code::MethodBody::new(self.class, data))
    }

    /// The declared checked exceptions.
    pub fn exceptions(
        self,
    ) -> impl ExactSizeIterator<Item = &'class ClassName> + DoubleEndedIterator {
        self.data
            .exceptions
            .iter()
            .map(|id| self.class.constant_pool.class_name(*id))
    }

    /// The method annotations.
    #[must_use]
    pub const fn annotations(&self) -> Annotations<'class> {
        Annotations::new(self.class, &self.data.annotations)
    }

    /// The method type annotations.
    #[must_use]
    pub const fn type_annotations(&self) -> TypeAnnotations<'class> {
        TypeAnnotations::new(self.class, &self.data.type_annotations)
    }

    /// The parameter annotations.
    #[must_use]
    pub const fn parameter_annotations(&self) -> ParameterAnnotations<'class> {
        ParameterAnnotations::new(self.class, &self.data.parameter_annotations)
    }

    /// The default annotation value, if present.
    #[must_use]
    pub fn annotation_default(&self) -> Option<annotation::ElementValue<'class>> {
        self.data
            .annotation_default
            .as_ref()
            .map(|data| annotation::ElementValue::new(self.class, data))
    }

    /// The method parameter information.
    pub fn parameters(
        self,
    ) -> impl ExactSizeIterator<Item = ParameterInfo<'class>> + DoubleEndedIterator {
        self.data.parameters.iter().map(move |data| ParameterInfo {
            class: self.class,
            data,
        })
    }

    /// Whether the method is synthetic.
    #[must_use]
    pub const fn is_synthetic(&self) -> bool {
        self.data.is_synthetic
    }

    /// Whether the method is deprecated.
    #[must_use]
    pub const fn is_deprecated(&self) -> bool {
        self.data.is_deprecated
    }

    /// The generic signature, if present.
    #[must_use]
    pub fn signature(&self) -> Option<&'class JavaStr> {
        self.data
            .signature
            .map(|id| self.class.constant_pool.utf8(id))
    }

    /// The unrecognized method attributes.
    pub fn other_attributes(
        self,
    ) -> impl ExactSizeIterator<Item = Attribute<'class>> + DoubleEndedIterator {
        self.data
            .other_attributes
            .iter()
            .map(|data| Attribute::new(self.class, data))
    }

    /// Returns a borrowed reference to this method.
    #[must_use]
    pub fn as_ref(&self) -> MethodRef<'class> {
        MethodRef {
            owner: self.class.reference_type(),
            name: self.name(),
            descriptor: self.descriptor(),
        }
    }

    /// The method of a static initializer block.
    pub const CLASS_INITIALIZER_NAME: &'static str = "<clinit>";
    /// The method of a constructor.
    pub const CONSTRUCTOR_NAME: &'static str = "<init>";

    /// Checks if the method is a constructor.
    #[must_use]
    pub fn is_constructor(&self) -> bool {
        self.name() == Self::CONSTRUCTOR_NAME
    }

    /// Checks if the method is a static initializer block.
    #[must_use]
    pub fn is_static_initializer_block(&self) -> bool {
        self.name() == Self::CLASS_INITIALIZER_NAME
    }
}

impl fmt::Debug for Method<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Method")
            .field("owner", &self.owner().name())
            .field("access_flags", &self.access_flags())
            .field("name", &self.name())
            .field("descriptor", &self.descriptor())
            .field("body", &self.body())
            .field("exceptions", &DebugList::new(|| self.exceptions()))
            .field("annotations", &self.annotations())
            .field("type_annotations", &self.type_annotations())
            .field("parameter_annotations", &self.parameter_annotations())
            .field("annotation_default", &self.annotation_default())
            .field("parameters", &DebugList::new(|| self.parameters()))
            .field("is_synthetic", &self.is_synthetic())
            .field("is_deprecated", &self.is_deprecated())
            .field("signature", &self.signature())
            .field(
                "other_attributes",
                &DebugList::new(|| self.other_attributes()),
            )
            .finish()
    }
}

/// The information of a method parameter.
#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(Clone, Copy)]
pub struct ParameterInfo<'class> {
    class: &'class Class,
    data: &'class ParameterInfoData,
}

#[derive(Debug, Clone)]
pub(crate) struct ParameterInfoData {
    pub name: Option<Utf8Id>,
    pub access_flags: ParameterAccessFlags,
}

impl<'class> ParameterInfo<'class> {
    /// The name of the parameter, if recorded.
    #[must_use]
    pub fn name(self) -> Option<&'class JavaStr> {
        self.data.name.map(|id| self.class.constant_pool.utf8(id))
    }

    /// The access flags of the parameter.
    #[must_use]
    pub const fn access_flags(self) -> ParameterAccessFlags {
        self.data.access_flags
    }
}

impl fmt::Debug for ParameterInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("ParameterInfo")
            .field("name", &self.name())
            .field("access_flags", &self.access_flags())
            .finish()
    }
}

bitflags! {
    /// Access flags for a [`Method`].
    #[doc = see_jvm_spec!(4, 6)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct AccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `private`; accessible only within the defining class and other classes belonging to the same nest.
        const PRIVATE = 0x0002;
        /// Declared `protected`; may be accessed within subclasses.
        const PROTECTED = 0x0004;
        /// Declared `static`.
        const STATIC = 0x0008;
        /// Declared `final`; must not be overridden.
        const FINAL = 0x0010;
        /// Declared `synchronized`; invocation is wrapped by a monitor use.
        const SYNCHRONIZED = 0x0020;
        /// A bridge method, generated by the compiler.
        const BRIDGE = 0x0040;
        /// Declared with variable number of arguments.
        const VARARGS = 0x0080;
        /// Declared `native`; implemented in a language other than Java.
        const NATIVE = 0x0100;
        /// Declared `abstract`; no implementation is provided.
        const ABSTRACT = 0x0400;
        /// In a `class` file whose major version is at least 46 and at most 60; Declared `strictfp`.
        const STRICT = 0x0800;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
    }
}

bitflags! {
    /// The access flags for a method parameter.
    #[doc = see_jvm_spec!(4, 7, 24)]
    #[derive(Debug, PartialEq, Eq, Clone, Copy)]
    pub struct ParameterAccessFlags: u16 {
        /// Declared `final`; may not be assigned to after initialization.
        const FINAL = 0x0010;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as either `mandated` or `optional`.
        const MANDATED = 0x8000;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parameter_names_borrow_the_class() {
        let mut class = Class::default();
        let name = class
            .constant_pool
            .insert_utf8(JavaString::from_cesu8(vec![0xed, 0xa0, 0x80]).unwrap());
        let data = ParameterInfoData {
            name: Some(name),
            access_flags: ParameterAccessFlags::empty(),
        };
        let parameter = ParameterInfo {
            class: &class,
            data: &data,
        };
        assert_eq!(parameter.name().unwrap().as_bytes(), &[0xed, 0xa0, 0x80]);
    }
}
