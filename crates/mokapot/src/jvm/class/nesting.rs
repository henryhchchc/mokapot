//! Views of nested and enclosing classes.

use std::fmt;

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Class, JavaStr,
        class_file::constant_pool::{ClassId, MethodNameAndTypeId, Utf8Id},
    },
    types::{class_name::ClassName, method_descriptor::MethodDescriptor},
};

use super::NestedClassAccessFlags;

#[derive(Debug, Clone)]
pub(crate) struct InnerClassData {
    pub(crate) inner_class: ClassId,
    pub(crate) outer_class: Option<ClassId>,
    pub(crate) inner_name: Option<Utf8Id>,
    pub(crate) access_flags: NestedClassAccessFlags,
}

/// Information about an inner class.
#[doc = see_jvm_spec!(4, 7, 6)]
#[derive(Clone, Copy)]
pub struct InnerClassInfo<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class InnerClassData,
}

#[derive(Debug, Clone)]
pub(crate) struct EnclosingMethodData {
    pub(crate) class: ClassId,
    pub(crate) method: Option<MethodNameAndTypeId>,
}

/// Information about the class and method enclosing a local or anonymous class.
#[doc = see_jvm_spec!(4, 7, 7)]
#[derive(Clone, Copy)]
pub struct EnclosingMethod<'class> {
    pub(crate) owner: &'class Class,
    pub(crate) data: &'class EnclosingMethodData,
}

impl<'class> InnerClassInfo<'class> {
    /// Returns the inner class.
    #[must_use]
    pub fn inner_class(&self) -> &'class ClassName {
        self.class.constant_pool.class_name(self.data.inner_class)
    }

    /// Returns the outer class, if present.
    #[must_use]
    pub fn outer_class(&self) -> Option<&'class ClassName> {
        self.data
            .outer_class
            .map(|id| self.class.constant_pool.class_name(id))
    }

    /// Returns the source-level name of the inner class, if present.
    #[must_use]
    pub fn inner_name(&self) -> Option<&'class JavaStr> {
        self.data
            .inner_name
            .map(|id| self.class.constant_pool.utf8(id))
    }

    /// Returns the inner class access flags.
    #[must_use]
    pub const fn access_flags(&self) -> NestedClassAccessFlags {
        self.data.access_flags
    }
}

impl<'class> EnclosingMethod<'class> {
    /// Returns the class that encloses this class.
    #[must_use]
    pub fn class(&self) -> &'class ClassName {
        self.owner.constant_pool.class_name(self.data.class)
    }

    /// Returns the enclosing method name and descriptor, if present.
    #[must_use]
    pub fn method_name_and_desc(&self) -> Option<(&'class JavaStr, &'class MethodDescriptor)> {
        self.data
            .method
            .map(|id| self.owner.constant_pool.method_name_and_type(id))
    }

    /// Returns the parsed class that owns this attribute.
    #[must_use]
    pub const fn owner(&self) -> &'class Class {
        self.owner
    }
}

impl fmt::Debug for InnerClassInfo<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("InnerClassInfo")
            .field("inner_class", self.inner_class())
            .field("outer_class", &self.outer_class())
            .field("inner_name", &self.inner_name())
            .field("access_flags", &self.access_flags())
            .finish()
    }
}

impl fmt::Debug for EnclosingMethod<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("EnclosingMethod")
            .field("class", self.class())
            .field("method_name_and_desc", &self.method_name_and_desc())
            .finish()
    }
}
