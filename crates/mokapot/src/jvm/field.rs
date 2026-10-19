//! JVM fields and constant values.

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Class, ConstantValue, JavaStr, JavaString,
        annotation::{
            AnnotationData, Annotations, AnnotationsData, TypeAnnotationData, TypeAnnotations,
        },
        attribute::{Attribute, AttributeData},
        class_file::constant_pool::{ConstantValueId, FieldTypeId, Utf8Id},
        references::FieldRef,
    },
    types::reference_type::ReferenceType,
};

/// A generic type signature for a field, a formal parameter, a local variable, or a record component.
pub type Signature = JavaString;

#[derive(Debug, Clone)]
pub(crate) struct FieldData {
    pub access_flags: AccessFlags,
    pub name: Utf8Id,
    pub field_type: FieldTypeId,
    pub constant_value: Option<ConstantValueId>,
    pub is_synthetic: bool,
    pub is_deprecated: bool,
    pub signature: Option<Utf8Id>,
    pub annotations: AnnotationsData<AnnotationData>,
    pub type_annotations: AnnotationsData<TypeAnnotationData>,
    pub other_attributes: Box<[AttributeData]>,
}

/// A field declared by a JVM class.
#[doc = see_jvm_spec!(4, 5)]
#[derive(Clone, Copy)]
pub struct Field<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class FieldData,
}

impl<'class> Field<'class> {
    /// The class declaring this field.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.class
    }

    /// The field access flags.
    #[must_use]
    pub const fn access_flags(&self) -> AccessFlags {
        self.data.access_flags
    }

    /// The field name.
    #[must_use]
    pub fn name(&self) -> &'class JavaStr {
        self.class.constant_pool.utf8(self.data.name)
    }

    /// The field type.
    #[must_use]
    pub fn field_type(&self) -> &'class crate::types::field_type::FieldType {
        self.class.constant_pool.field_type(self.data.field_type)
    }

    /// The constant value, if present.
    #[must_use]
    pub fn constant_value(&self) -> Option<&'class ConstantValue> {
        self.data
            .constant_value
            .map(|id| self.class.constant_pool.constant_value(id))
    }

    /// Whether this field is static.
    #[must_use]
    pub const fn is_static(&self) -> bool {
        self.data.access_flags.contains(AccessFlags::STATIC)
    }

    /// Whether this field is synthetic.
    #[must_use]
    pub const fn is_synthetic(&self) -> bool {
        self.data.is_synthetic
    }

    /// Whether this field is deprecated.
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

    /// The field annotations.
    #[must_use]
    pub const fn annotations(&self) -> Annotations<'class> {
        Annotations::new(self.class, &self.data.annotations)
    }

    /// The field type annotations.
    #[must_use]
    pub const fn type_annotations(&self) -> TypeAnnotations<'class> {
        TypeAnnotations::new(self.class, &self.data.type_annotations)
    }

    /// The unrecognized field attributes.
    pub fn other_attributes(
        self,
    ) -> impl ExactSizeIterator<Item = Attribute<'class>> + DoubleEndedIterator {
        self.data
            .other_attributes
            .iter()
            .map(|data| Attribute::new(self.class, data))
    }

    /// Creates a [`FieldRef`] referring to this field.
    #[must_use]
    pub fn as_ref(&self) -> FieldRef {
        FieldRef {
            owner: ReferenceType::Class(self.owner().name().clone()),
            name: self.name().to_owned(),
            field_type: self.field_type().clone(),
        }
    }
}

impl std::fmt::Debug for Field<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Field")
            .field("owner", &self.owner().name())
            .field("access_flags", &self.access_flags())
            .field("name", &self.name())
            .field("field_type", &self.field_type())
            .field("constant_value", &self.constant_value())
            .field("is_synthetic", &self.is_synthetic())
            .field("is_deprecated", &self.is_deprecated())
            .field("signature", &self.signature())
            .field("annotations", &self.annotations())
            .field("type_annotations", &self.type_annotations())
            .field(
                "other_attributes",
                &self.other_attributes().collect::<Vec<_>>(),
            )
            .finish()
    }
}

bitflags::bitflags! {
    /// The access flags of a field.
    #[doc = see_jvm_spec!(4, 5)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct AccessFlags: u16 {
        /// Declared `public`; may be accessed from outside its package.
        const PUBLIC = 0x0001;
        /// Declared `private`; accessible only within its class and nest.
        const PRIVATE = 0x0002;
        /// Declared `protected`; accessible within subclasses.
        const PROTECTED = 0x0004;
        /// Declared `static`.
        const STATIC = 0x0008;
        /// Declared `final`; never directly assigned to after object construction.
        const FINAL = 0x0010;
        /// Declared `volatile`; cannot be cached.
        const VOLATILE = 0x0040;
        /// Declared `transient`; not written or read by a persistent object manager.
        const TRANSIENT = 0x0080;
        /// Declared synthetic; not present in the source code.
        const SYNTHETIC = 0x1000;
        /// Declared as an element of an `enum` class.
        const ENUM = 0x4000;
    }
}
