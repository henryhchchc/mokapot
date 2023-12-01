use super::references::{ClassReference, ModuleReference, PackageReference};
use bitflags::bitflags;

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub flags: ModuleFlags,
    pub version: Option<String>,
    pub requires: Vec<ModuleRequire>,
    pub exports: Vec<ModuleExport>,
    pub opens: Vec<ModuleOpen>,
    pub uses: Vec<ClassReference>,
    pub provides: Vec<ModuleProvide>,
}

#[derive(Debug)]
pub struct ModuleProvide {
    pub service: ClassReference,
    pub with: Vec<ClassReference>,
}
#[derive(Debug)]
pub struct ModuleOpen {
    pub package: PackageReference,
    pub flags: ModuleOpenFlags,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleExport {
    pub package: PackageReference,
    pub flags: ModuleExportFlags,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleRequire {
    pub module: ModuleReference,
    pub flags: ModuleRequireFlags,
    pub version: Option<String>,
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
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
    #[derive(Debug, PartialEq, Eq)]
    pub struct ModuleExportFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}

bitflags! {
    #[derive(Debug, PartialEq, Eq)]
    pub struct ModuleOpenFlags: u16 {
        /// Indicates that this opening was not explicitly or implicitly declared in the source of the module declaration.
        const SYNTHETIC = 0x1000;
        /// Indicates that this opening was implicitly declared in the source of the module declaration.
        const MANDATED = 0x8000;
    }
}
