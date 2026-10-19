//! Unrecognized JVM attributes.

use crate::jvm::{Class, JavaStr, class_file::constant_pool::Utf8Id};

#[derive(Debug, Clone)]
pub(crate) struct AttributeData {
    pub name: Utf8Id,
    pub bytes: Box<[u8]>,
}

/// An unrecognized attribute attached to a JVM class-file element.
#[derive(Clone, Copy)]
pub struct Attribute<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class AttributeData,
}

impl<'class> Attribute<'class> {
    pub(crate) const fn new(class: &'class Class, data: &'class AttributeData) -> Self {
        Self { class, data }
    }

    /// The class containing this attribute.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.class
    }

    /// The attribute name.
    #[must_use]
    pub fn name(&self) -> &'class JavaStr {
        self.class.constant_pool.utf8(self.data.name)
    }

    /// The attribute payload bytes.
    #[must_use]
    pub fn bytes(&self) -> &'class [u8] {
        &self.data.bytes
    }
}

impl PartialEq for Attribute<'_> {
    fn eq(&self, other: &Self) -> bool {
        self.name() == other.name() && self.bytes() == other.bytes()
    }
}

impl Eq for Attribute<'_> {}

impl std::fmt::Debug for Attribute<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Attribute")
            .field("owner", &self.owner().name())
            .field("name", &self.name())
            .field("bytes", &self.bytes())
            .finish()
    }
}

impl Class {
    /// The unrecognized class attributes.
    pub fn other_attributes(
        &self,
    ) -> impl ExactSizeIterator<Item = Attribute<'_>> + DoubleEndedIterator {
        self.other_attributes
            .iter()
            .map(|data| Attribute::new(self, data))
    }
}
