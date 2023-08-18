use crate::{
    elements::{
        class_file::{ClassFileParsingResult, ClassReference},
        constant_pool::ConstantPool,
    },
    utils::{read_u16, read_u32},
};

use super::Attribute;

#[derive(Debug)]
pub struct ModuleReference {
    pub name: String,
}

#[derive(Debug)]
pub struct PackageReference {
    pub name: String,
}

#[derive(Debug)]
pub struct ModuleRequire {
    pub module: ModuleReference,
    pub flags: u16,
    pub version: Option<String>,
}
impl ModuleRequire {
    fn parse_multiple<R>(
        reader: &mut R,
        requires_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<ModuleRequire>>
    where
        R: std::io::Read,
    {
        let mut requires = Vec::with_capacity(requires_count as usize);
        for _ in 0..requires_count {
            let module_index = read_u16(reader)?;
            let module = constant_pool.get_module_reference(module_index)?;
            let flags = read_u16(reader)?;
            let version_index = read_u16(reader)?;
            let version = if version_index > 0 {
                Some(constant_pool.get_string(version_index)?)
            } else {
                None
            };
            requires.push(ModuleRequire {
                module,
                flags,
                version,
            })
        }
        Ok(requires)
    }
}

#[derive(Debug)]
pub struct ModuleExport {
    pub package: PackageReference,
    pub flags: u16,
    pub to: Vec<ModuleReference>,
}
impl ModuleExport {
    fn parse_multiple<R>(
        reader: &mut R,
        exports_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<ModuleExport>>
    where
        R: std::io::Read,
    {
        let mut exports = Vec::with_capacity(exports_count as usize);
        for _ in 0..exports_count {
            let package_index = read_u16(reader)?;
            let package = constant_pool.get_package_reference(package_index)?;
            let flags = read_u16(reader)?;
            let to_count = read_u16(reader)?;
            let mut to = Vec::with_capacity(to_count as usize);
            for _ in 0..to_count {
                let module_index = read_u16(reader)?;
                let module = constant_pool.get_module_reference(module_index)?;
                to.push(module);
            }
            exports.push(ModuleExport { package, flags, to });
        }
        Ok(exports)
    }
}

#[derive(Debug)]
pub struct ModuleOpen {
    pub package: PackageReference,
    pub flags: u16,
    pub to: Vec<ModuleReference>,
}
impl ModuleOpen {
    fn parse_multiple<R>(
        reader: &mut R,
        opens_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<ModuleOpen>>
    where
        R: std::io::Read,
    {
        let mut opens = Vec::with_capacity(opens_count as usize);
        for _ in 0..opens_count {
            let package_index = read_u16(reader)?;
            let package = constant_pool.get_package_reference(package_index)?;
            let flags = read_u16(reader)?;
            let to_count = read_u16(reader)?;
            let mut to = Vec::with_capacity(to_count as usize);
            for _ in 0..to_count {
                let module_index = read_u16(reader)?;
                let module = constant_pool.get_module_reference(module_index)?;
                to.push(module);
            }
            opens.push(ModuleOpen { package, flags, to });
        }
        Ok(opens)
    }
}

#[derive(Debug)]
pub struct ModuleProvide {
    pub service: ClassReference,
    pub with: Vec<ClassReference>,
}
impl ModuleProvide {
    fn parse_multiple<R>(
        reader: &mut R,
        provides_count: u16,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Vec<ModuleProvide>>
    where
        R: std::io::Read,
    {
        let mut provides = Vec::with_capacity(provides_count as usize);
        for _ in 0..provides_count {
            let service_index = read_u16(reader)?;
            let service = constant_pool.get_class_ref(service_index)?;
            let with_count = read_u16(reader)?;
            let mut with = Vec::with_capacity(with_count as usize);
            for _ in 0..with_count {
                let provider_idx = read_u16(reader)?;
                let provider = constant_pool.get_class_ref(provider_idx)?;
                with.push(provider);
            }
            provides.push(ModuleProvide { service, with });
        }
        Ok(provides)
    }
}

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

impl Attribute {
    pub(super) fn parse_module<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let name_index = read_u16(reader)?;
        let name = constant_pool.get_string(name_index)?;
        let flags = read_u16(reader)?;
        let version_index = read_u16(reader)?;
        let version = if version_index > 0 {
            Some(constant_pool.get_string(version_index)?)
        } else {
            None
        };
        let requires_count = read_u16(reader)?;
        let requires = ModuleRequire::parse_multiple(reader, requires_count, constant_pool)?;
        let exports_count = read_u16(reader)?;
        let exports = ModuleExport::parse_multiple(reader, exports_count, constant_pool)?;
        let opens_count = read_u16(reader)?;
        let opens = ModuleOpen::parse_multiple(reader, opens_count, constant_pool)?;
        let uses_count = read_u16(reader)?;
        let mut uses = Vec::with_capacity(uses_count as usize);
        for _ in 0..uses_count {
            let class_index = read_u16(reader)?;
            let class = constant_pool.get_class_ref(class_index)?;
            uses.push(class);
        }
        let provides_count = read_u16(reader)?;
        let provides = ModuleProvide::parse_multiple(reader, provides_count, constant_pool)?;
        Ok(Self::Module(Module {
            name,
            flags,
            version,
            requires,
            exports,
            opens,
            uses,
            provides,
        }))
    }
    pub(super) fn parse_module_packages<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        let _attribute_length = read_u32(reader)?;
        let package_count = read_u16(reader)?;
        let mut packages = Vec::with_capacity(package_count as usize);
        for _ in 0..package_count {
            let package_index = read_u16(reader)?;
            let package = constant_pool.get_package_reference(package_index)?;
            packages.push(package);
        }
        Ok(Self::ModulePackages(packages))
    }
    pub(super) fn parse_module_main_class<R>(
        reader: &mut R,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self>
    where
        R: std::io::Read,
    {
        Self::check_attribute_length(reader, 2)?;
        let main_class_index = read_u16(reader)?;
        let main_class = constant_pool.get_class_ref(main_class_index)?;
        Ok(Self::ModuleMainClass(main_class))
    }
}
