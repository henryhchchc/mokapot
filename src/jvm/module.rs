//! Modules in the JPMS.
use bitflags::bitflags;

use super::class::ClassReference;

/// A JVM module.
/// See the [JVM Specification §4.7.25](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.25) for more information.
#[derive(Debug, Clone)]
pub struct Module {
    /// The name of the module.
    pub name: String,
    /// The flags of the module.
    pub flags: ModuleFlags,
    /// The version of the module.
    pub version: Option<String>,
    /// A list of the modules that are required by this module.
    pub requires: Vec<ModuleRequire>,
    /// A list of the modules that are exported by this module.
    pub exports: Vec<ModuleExport>,
    /// A list of the modules that are opened by this module.
    pub opens: Vec<ModuleOpen>,
    /// A list of the classes that are used by this module.
    pub uses: Vec<ClassReference>,
    /// A list of the services that are provided by this module.
    pub provides: Vec<ModuleProvide>,
}

/// A service provided by a module.
/// See the [JVM Specification §4.7.25](https://docs.oracle.com/javase/specs/jvms/se21/html/jvms-4.html#jvms-4.7.25) for more information.
#[derive(Debug, Clone)]
pub struct ModuleProvide {
    /// The reference to a class which is provided as a service.
    pub service: ClassReference,
    /// The list of the classes which implement the service.
    pub with: Vec<ClassReference>,
}

/// A module opening.
#[derive(Debug, Clone)]
pub struct ModuleOpen {
    /// The reference to the package which is opened.
    pub package: PackageReference,
    /// The flags of the opening.
    pub flags: ModuleOpenFlags,
    /// The list of the modules which can access the package.
    pub to: Vec<ModuleReference>,
}

/// A module export.
#[derive(Debug, Clone)]
pub struct ModuleExport {
    /// The reference to the package which is exported.
    pub package: PackageReference,
    /// The flags of the export.
    pub flags: ModuleExportFlags,
    /// The list of the modules which can access the package.
    pub to: Vec<ModuleReference>,
}

/// A module require.
#[derive(Debug, Clone)]
pub struct ModuleRequire {
    /// The reference to the module which is required.
    pub module: ModuleReference,
    /// The flags of the require.
    pub flags: ModuleRequireFlags,
    /// The version of the required module.
    pub version: Option<String>,
}

bitflags! {
    /// The flags of a module.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModuleFlags: u16 {
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
    pub struct ModuleRequireFlags: u16 {
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
    pub struct ModuleExportFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}

bitflags! {
    /// The flags of a module open.
    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct ModuleOpenFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}

/// A reference to a module in the binary format.
#[derive(Debug, Clone)]
pub struct ModuleReference {
    /// The name of the module.
    pub name: String,
}

/// A reference to a package in the binary format.
#[derive(Debug, Clone)]
pub struct PackageReference {
    /// The binary name of the package.
    pub binary_name: String,
}
