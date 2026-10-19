//! Modules in the JPMS.

use std::fmt;

use bitflags::bitflags;

use crate::{
    intrinsics::see_jvm_spec,
    jvm::{
        Class, DebugList, JavaStr,
        constant_pool::{ClassId, ModuleId, PackageId, Utf8Id},
        references::ModuleRef,
    },
    types::{class_name::ClassName, package_name::PackageName},
};

/// A module declared by a class.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy)]
pub struct Module<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class ModuleData,
}

#[derive(Debug, Clone)]
pub(crate) struct ModuleData {
    pub(crate) name: ModuleId,
    pub(crate) flags: Flags,
    pub(crate) version: Option<Utf8Id>,
    pub(crate) requires: Box<[RequireData]>,
    pub(crate) exports: Box<[ExportData]>,
    pub(crate) opens: Box<[OpenData]>,
    pub(crate) uses: Box<[ClassId]>,
    pub(crate) provides: Box<[ProvideData]>,
}

#[derive(Debug, Clone)]
pub(crate) struct RequireData {
    pub(crate) module: ModuleId,
    pub(crate) flags: RequireFlags,
    pub(crate) version: Option<Utf8Id>,
}

#[derive(Debug, Clone)]
pub(crate) struct ExportData {
    pub(crate) package: PackageId,
    pub(crate) flags: ExportFlags,
    pub(crate) to: Box<[ModuleId]>,
}

#[derive(Debug, Clone)]
pub(crate) struct OpenData {
    pub(crate) package: PackageId,
    pub(crate) flags: OpenFlags,
    pub(crate) to: Box<[ModuleId]>,
}

#[derive(Debug, Clone)]
pub(crate) struct ProvideData {
    pub(crate) service: ClassId,
    pub(crate) with: Box<[ClassId]>,
}

/// A module require.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy)]
pub struct Require<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class RequireData,
}

/// A module export.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy)]
pub struct Export<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class ExportData,
}

/// A module opening.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy)]
pub struct Open<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class OpenData,
}

/// A service provided by a module.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Clone, Copy)]
pub struct Provide<'class> {
    pub(crate) class: &'class Class,
    pub(crate) data: &'class ProvideData,
}

impl<'class> Module<'class> {
    /// Returns the module name.
    #[must_use]
    pub fn name(&self) -> &'class JavaStr {
        self.class
            .constant_pool
            .module_ref(self.data.name)
            .name
            .as_java_str()
    }

    /// Returns the module flags.
    #[must_use]
    pub const fn flags(&self) -> Flags {
        self.data.flags
    }

    /// Returns the module version, if present.
    #[must_use]
    pub fn version(&self) -> Option<&'class JavaStr> {
        self.data
            .version
            .map(|id| self.class.constant_pool.utf8(id))
    }

    /// Returns the modules required by this module.
    pub fn requires(self) -> impl ExactSizeIterator<Item = Require<'class>> + DoubleEndedIterator {
        self.data.requires.iter().map(|data| Require {
            class: self.class,
            data,
        })
    }

    /// Returns the packages exported by this module.
    pub fn exports(self) -> impl ExactSizeIterator<Item = Export<'class>> + DoubleEndedIterator {
        self.data.exports.iter().map(|data| Export {
            class: self.class,
            data,
        })
    }

    /// Returns the packages opened by this module.
    pub fn opens(self) -> impl ExactSizeIterator<Item = Open<'class>> + DoubleEndedIterator {
        self.data.opens.iter().map(|data| Open {
            class: self.class,
            data,
        })
    }

    /// Returns the service classes used by this module.
    pub fn uses(self) -> impl ExactSizeIterator<Item = &'class ClassName> + DoubleEndedIterator {
        self.data
            .uses
            .iter()
            .map(|id| self.class.constant_pool.class_name(*id))
    }

    /// Returns the services provided by this module.
    pub fn provides(self) -> impl ExactSizeIterator<Item = Provide<'class>> + DoubleEndedIterator {
        self.data.provides.iter().map(|data| Provide {
            class: self.class,
            data,
        })
    }
}

impl<'class> Require<'class> {
    /// Returns the required module.
    #[must_use]
    pub fn module(&self) -> &'class ModuleRef {
        self.class.constant_pool.module_ref(self.data.module)
    }

    /// Returns the require flags.
    #[must_use]
    pub const fn flags(&self) -> RequireFlags {
        self.data.flags
    }

    /// Returns the required module's version, if present.
    #[must_use]
    pub fn version(&self) -> Option<&'class JavaStr> {
        self.data
            .version
            .map(|id| self.class.constant_pool.utf8(id))
    }
}

impl<'class> Export<'class> {
    /// Returns the exported package.
    #[must_use]
    pub fn package(&self) -> &'class PackageName {
        self.class.constant_pool.package_name(self.data.package)
    }

    /// Returns the export flags.
    #[must_use]
    pub const fn flags(&self) -> ExportFlags {
        self.data.flags
    }

    /// Returns the modules to which this package is exported.
    pub fn to(self) -> impl ExactSizeIterator<Item = &'class ModuleRef> + DoubleEndedIterator {
        self.data
            .to
            .iter()
            .map(|id| self.class.constant_pool.module_ref(*id))
    }
}

impl<'class> Open<'class> {
    /// Returns the opened package.
    #[must_use]
    pub fn package(&self) -> &'class PackageName {
        self.class.constant_pool.package_name(self.data.package)
    }

    /// Returns the open flags.
    #[must_use]
    pub const fn flags(&self) -> OpenFlags {
        self.data.flags
    }

    /// Returns the modules to which this package is opened.
    pub fn to(self) -> impl ExactSizeIterator<Item = &'class ModuleRef> + DoubleEndedIterator {
        self.data
            .to
            .iter()
            .map(|id| self.class.constant_pool.module_ref(*id))
    }
}

impl<'class> Provide<'class> {
    /// Returns the service class.
    #[must_use]
    pub fn service(&self) -> &'class ClassName {
        self.class.constant_pool.class_name(self.data.service)
    }

    /// Returns the classes that implement the service.
    pub fn with(self) -> impl ExactSizeIterator<Item = &'class ClassName> + DoubleEndedIterator {
        self.data
            .with
            .iter()
            .map(|id| self.class.constant_pool.class_name(*id))
    }
}

impl fmt::Debug for Module<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Module")
            .field("name", &self.name())
            .field("flags", &self.flags())
            .field("version", &self.version())
            .field("requires", &DebugList::new(|| self.requires()))
            .field("exports", &DebugList::new(|| self.exports()))
            .field("opens", &DebugList::new(|| self.opens()))
            .field("uses", &DebugList::new(|| self.uses()))
            .field("provides", &DebugList::new(|| self.provides()))
            .finish()
    }
}

impl fmt::Debug for Require<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Require")
            .field("module", self.module())
            .field("flags", &self.flags())
            .field("version", &self.version())
            .finish()
    }
}

impl fmt::Debug for Export<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Export")
            .field("package", self.package())
            .field("flags", &self.flags())
            .field("to", &DebugList::new(|| self.to()))
            .finish()
    }
}

impl fmt::Debug for Open<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Open")
            .field("package", self.package())
            .field("flags", &self.flags())
            .field("to", &DebugList::new(|| self.to()))
            .finish()
    }
}

impl fmt::Debug for Provide<'_> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Provide")
            .field("service", self.service())
            .field("with", &DebugList::new(|| self.with()))
            .finish()
    }
}

bitflags! {
    /// The flags of a module.
    #[doc = see_jvm_spec!(4, 7, 25)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct Flags: u16 {
        /// Indicates that this module is open.
        const OPEN = 0x0020;
        /// Indicates that this module was not explicitly or implicitly declared.
        const SYNTHETIC = 0x1000;
        /// Indicates that this module is implicitly declared.
        const MANDATED = 0x8000;
    }
}

bitflags! {
    /// The flags of a module require.
    #[doc = see_jvm_spec!(4, 7, 25)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct RequireFlags: u16 {
        /// Indicates that any module which depends on the current module, implicitly declares a dependence on the module indicated by this entry.
        const TRANSITIVE = 0x0020;
        /// Indicates that the module indicated by this entry can be read by the current module at compile time, despite not being observable at run time.
        const STATIC_PHASE = 0x0040;
        /// Indicates that the dependence indicated by this entry is mandatory at run time.
        const SYNTHETIC = 0x1000;
        /// Indicates that the dependence indicated by this entry was implicitly declared in source code, such as by an import statement.
        const MANDATED = 0x8000;
    }
}

bitflags! {
    /// The flags of a module export.
    #[doc = see_jvm_spec!(4, 7, 25)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ExportFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}

bitflags! {
    /// The flags of a module open.
    #[doc = see_jvm_spec!(4, 7, 25)]
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}
