//! Borrowed accessors for class-file metadata.

use crate::{
    jvm::{Class, JavaStr},
    types::{class_name::ClassName, package_name::PackageName},
};

impl Class {
    /// Returns this class's name.
    #[must_use]
    pub fn name(&self) -> &ClassName {
        self.constant_pool.class_name(self.name)
    }

    /// Returns the superclass, if this class has one.
    #[must_use]
    pub fn super_class(&self) -> Option<&ClassName> {
        self.super_class.map(|id| self.constant_pool.class_name(id))
    }

    /// Returns the interfaces implemented by this class.
    pub fn interfaces(&self) -> impl ExactSizeIterator<Item = &ClassName> + DoubleEndedIterator {
        self.interfaces
            .iter()
            .map(|id| self.constant_pool.class_name(*id))
    }

    /// Returns the source file name, if present.
    #[must_use]
    pub fn source_file(&self) -> Option<&JavaStr> {
        self.source_file.map(|id| self.constant_pool.utf8(id))
    }

    /// Returns the module packages declared by this class, if any.
    pub fn module_packages(
        &self,
    ) -> impl ExactSizeIterator<Item = &PackageName> + DoubleEndedIterator {
        self.module_packages
            .iter()
            .map(|id| self.constant_pool.package_name(*id))
    }

    /// Returns the module's main class, if present.
    #[must_use]
    pub fn module_main_class(&self) -> Option<&ClassName> {
        self.module_main_class
            .map(|id| self.constant_pool.class_name(id))
    }

    /// Returns the enclosing nest host, if present.
    #[must_use]
    pub fn nest_host(&self) -> Option<&ClassName> {
        self.nest_host.map(|id| self.constant_pool.class_name(id))
    }

    /// Returns the classes in this class's nest.
    pub fn nest_members(&self) -> impl ExactSizeIterator<Item = &ClassName> + DoubleEndedIterator {
        self.nest_members
            .iter()
            .map(|id| self.constant_pool.class_name(*id))
    }

    /// Returns the subclasses permitted by this sealed class or interface.
    pub fn permitted_subclasses(
        &self,
    ) -> impl ExactSizeIterator<Item = &ClassName> + DoubleEndedIterator {
        self.permitted_subclasses
            .iter()
            .map(|id| self.constant_pool.class_name(*id))
    }

    /// Returns the generic signature, if present.
    #[must_use]
    pub fn signature(&self) -> Option<&JavaStr> {
        self.signature.map(|id| self.constant_pool.utf8(id))
    }
}
