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
    pub flags: ModuleFlags,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleExport {
    pub package: PackageReference,
    pub flags: ModuleFlags,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleRequire {
    pub module: ModuleReference,
    pub flags: ModuleFlags,
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
