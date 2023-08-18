use super::references::{ClassReference, ModuleReference, PackageReference};

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub flags: u16,
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
    pub flags: u16,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleExport {
    pub package: PackageReference,
    pub flags: u16,
    pub to: Vec<ModuleReference>,
}
#[derive(Debug)]
pub struct ModuleRequire {
    pub module: ModuleReference,
    pub flags: u16,
    pub version: Option<String>,
}
