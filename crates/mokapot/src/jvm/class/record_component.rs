//! JVM record components.

use std::{fmt, iter::FusedIterator};

use crate::jvm::{
    Class, DebugList, JavaStr,
    annotation::{
        AnnotationData, Annotations, AnnotationsData, TypeAnnotationData, TypeAnnotations,
    },
    attribute::{Attribute, AttributeData},
    constant_pool::{FieldTypeId, Utf8Id},
};
use crate::types::field_type::FieldType;

#[derive(Debug, Clone)]
pub(crate) struct RecordComponentData {
    pub name: Utf8Id,
    pub component_type: FieldTypeId,
    pub signature: Option<Utf8Id>,
    pub annotations: AnnotationsData<AnnotationData>,
    pub type_annotations: AnnotationsData<TypeAnnotationData>,
    pub other_attributes: Box<[AttributeData]>,
}

/// A record component declared by a JVM class.
#[derive(Clone, Copy)]
pub struct RecordComponent<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class RecordComponentData,
}

impl<'class> RecordComponent<'class> {
    /// The class declaring this record component.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.class
    }

    /// The component name.
    #[must_use]
    pub fn name(&self) -> &'class JavaStr {
        self.class.constant_pool.utf8(self.data.name)
    }

    /// The component type.
    #[must_use]
    pub fn component_type(&self) -> &'class FieldType {
        self.class
            .constant_pool
            .field_type(self.data.component_type)
    }

    /// The generic signature, if present.
    #[must_use]
    pub fn signature(&self) -> Option<&'class JavaStr> {
        self.data
            .signature
            .map(|id| self.class.constant_pool.utf8(id))
    }

    /// The component annotations.
    #[must_use]
    pub const fn annotations(&self) -> Annotations<'class> {
        Annotations::new(self.class, &self.data.annotations)
    }

    /// The component type annotations.
    #[must_use]
    pub const fn type_annotations(&self) -> TypeAnnotations<'class> {
        TypeAnnotations::new(self.class, &self.data.type_annotations)
    }

    /// The unrecognized record component attributes.
    pub fn other_attributes(
        self,
    ) -> impl ExactSizeIterator<Item = Attribute<'class>> + DoubleEndedIterator + FusedIterator + Clone
    {
        self.data
            .other_attributes
            .iter()
            .map(|data| Attribute::new(self.class, data))
    }
}

impl fmt::Debug for RecordComponent<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RecordComponent")
            .field("owner", &self.owner().name())
            .field("name", &self.name())
            .field("component_type", &self.component_type())
            .field("signature", &self.signature())
            .field("annotations", &self.annotations())
            .field("type_annotations", &self.type_annotations())
            .field(
                "other_attributes",
                &DebugList::new(|| self.other_attributes()),
            )
            .finish()
    }
}
