//! Modules in the JPMS.
use bitflags::bitflags;

use crate::macros::see_jvm_spec;

use super::references::{ClassRef, ModuleRef, PackageRef};

/// A JVM module.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Module {
    /// The name of the module.
    pub name: String,
    /// The flags of the module.
    pub flags: Flags,
    /// The version of the module.
    pub version: Option<String>,
    /// A list of the modules that are required by this module.
    pub requires: Vec<Require>,
    /// A list of the modules that are exported by this module.
    pub exports: Vec<Export>,
    /// A list of the modules that are opened by this module.
    pub opens: Vec<Open>,
    /// A list of the classes that are used by this module.
    pub uses: Vec<ClassRef>,
    /// A list of the services that are provided by this module.
    pub provides: Vec<Provide>,
}

/// A service provided by a module.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Provide {
    /// The reference to a class which is provided as a service.
    pub service: ClassRef,
    /// The list of the classes which implement the service.
    pub with: Vec<ClassRef>,
}

/// A module opening.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Open {
    /// The reference to the package which is opened.
    pub package: PackageRef,
    /// The flags of the opening.
    pub flags: OpenFlags,
    /// The list of the modules which can access the package.
    pub to: Vec<ModuleRef>,
}

/// A module export.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Export {
    /// The reference to the package which is exported.
    pub package: PackageRef,
    /// The flags of the export.
    pub flags: ExportFlags,
    /// The list of the modules which can access the package.
    pub to: Vec<ModuleRef>,
}

/// A module require.
#[doc = see_jvm_spec!(4, 7, 25)]
#[derive(Debug, Clone)]
pub struct Require {
    /// The reference to the module which is required.
    pub module: ModuleRef,
    /// The flags of the require.
    pub flags: RequireFlags,
    /// The version of the required module.
    pub version: Option<String>,
}

bitflags! {
    /// The flags of a module.
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
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct OpenFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}
