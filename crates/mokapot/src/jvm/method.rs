//! JVM methods.

use bitflags::bitflags;

use super::{
    Annotations, Class, JavaStr, JavaString, Method, annotation,
    annotation::{
        AnnotationData, AnnotationsData, ElementValueData, ParameterAnnotations,
        TypeAnnotationData, TypeAnnotations,
    },
    attribute::{Attribute, AttributeData},
    class_file::constant_pool::{ClassId, MethodDescriptorId, Utf8Id},
    code,
    references::MethodRef,
};
use crate::intrinsics::see_jvm_spec;
use crate::types::reference_type::ReferenceType;
use crate::types::{class_name::ClassName, method_descriptor::MethodDescriptor};

/// A generic type signature for a method.
pub type Signature = JavaString;

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
    pub parameters: Box<[ParameterInfo]>,
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
    #[must_use]
    pub fn parameters(&self) -> &'class [ParameterInfo] {
        &self.data.parameters
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

    /// Creates a [`MethodRef`] pointting to this method.
    #[must_use]
    pub fn as_ref(&self) -> MethodRef {
        MethodRef {
            owner: ReferenceType::Class(self.owner().name().clone()),
            name: self.name().to_owned(),
            descriptor: self.descriptor().clone(),
        }
    }
}

impl std::fmt::Debug for Method<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Method")
            .field("owner", &self.owner().name())
            .field("access_flags", &self.access_flags())
            .field("name", &self.name())
            .field("descriptor", &self.descriptor())
            .field("body", &self.body())
            .field("exceptions", &self.exceptions().collect::<Vec<_>>())
            .field("annotations", &self.annotations())
            .field("type_annotations", &self.type_annotations())
            .field("parameter_annotations", &self.parameter_annotations())
            .field("annotation_default", &self.annotation_default())
            .field("parameters", &self.parameters())
            .field("is_synthetic", &self.is_synthetic())
            .field("is_deprecated", &self.is_deprecated())
            .field("signature", &self.signature())
            .field(
                "other_attributes",
                &self.other_attributes().collect::<Vec<_>>(),
            )
            .finish()
    }
}

/// The information of a method parameter.
#[doc = see_jvm_spec!(4, 7, 24)]
#[derive(Debug, Clone)]
pub struct ParameterInfo {
    /// The name of the parameter.
    pub name: Option<String>,
    /// The access flags of the parameter.
    pub access_flags: ParameterAccessFlags,
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
    use crate::jvm::code::Instruction;
    use crate::types::Descriptor;

    #[test]
    fn pool_backed_methods_preserve_jvm_names_after_input_is_dropped() {
        let name = &[0xed, 0xa0, 0x80];
        let descriptor = b"()Lpkg/\xed\xa0\x80;";
        let class = {
            let mut bytes = vec![0xca, 0xfe, 0xba, 0xbe, 0, 0, 0, 61, 0, 6];
            for value in [b"java/lang/Object".as_slice(), name, descriptor, b"Code"] {
                if value == name {
                    bytes.extend([7, 0, 1]);
                }
                bytes.push(1);
                bytes.extend(u16::try_from(value.len()).unwrap().to_be_bytes());
                bytes.extend(value);
            }
            bytes.extend([
                0, 0x21, 0, 2, 0, 0, 0, 0, 0, 0, 0, 1, 0, 9, 0, 3, 0, 4, 0, 1, 0, 5, 0, 0, 0, 14,
                0, 1, 0, 0, 0, 0, 0, 2, 1, 0xb0, 0, 0, 0, 0, 0, 0,
            ]);
            Class::from_bytes(&bytes).unwrap()
        };
        let method = class.methods().next().unwrap();
        assert_eq!(method.name().as_bytes(), name);
        assert_eq!(method.descriptor().descriptor().as_bytes(), descriptor);
        let reference = method.as_ref();
        let ir = crate::ir::MokaIRMethod::from_method(method).unwrap();
        drop(class);
        assert_eq!(reference.name.as_bytes(), name);
        assert_eq!(ir.name.as_bytes(), name);
        assert_eq!(ir.descriptor.descriptor().as_bytes(), descriptor);
    }

    #[test]
    fn method_views_use_the_containing_class() {
        let fixture = crate::tests::method(
            [(0, Instruction::Return)],
            "()V",
            vec![],
            AccessFlags::PUBLIC,
        );
        let mut class = fixture.as_method().owner().clone();
        class.name = class
            .constant_pool
            .insert_class_name("Other".parse().unwrap());
        let descriptor: MethodDescriptor = "()V".parse().unwrap();
        let method = class.get_method_str("test", &descriptor).unwrap();
        assert!(std::ptr::eq(method.owner(), &raw const class));
        assert_eq!(
            method.as_ref().owner,
            ReferenceType::Class(class.name().clone())
        );
        assert!(class.get_method_str("missing", &descriptor).is_none());
        assert!(
            class
                .get_method_str("test", "()I".parse::<MethodDescriptor>().unwrap())
                .is_none()
        );
        let body = class.methods().next().unwrap().body().unwrap();
        assert_eq!(
            body.instructions().get(&0.into()),
            Some(&Instruction::Return)
        );
    }

    #[test]
    fn special_method_checks_use_only_the_name() {
        let mut method = crate::tests::method(
            [(0, Instruction::Return)],
            "()V",
            vec![],
            AccessFlags::PUBLIC,
        );
        for (name, constructor, initializer) in [
            (Method::CONSTRUCTOR_NAME, true, false),
            (Method::CLASS_INITIALIZER_NAME, false, true),
            ("ordinary", false, false),
        ] {
            method.set_name(name);
            assert_eq!(
                (
                    method.as_method().is_constructor(),
                    method.as_method().is_static_initializer_block()
                ),
                (constructor, initializer),
            );
        }
    }
}
