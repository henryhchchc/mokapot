use crate::{
    jvm::class::ClassFileParsingError,
    jvm::module::{
        Module, ModuleExport, ModuleExportFlags, ModuleFlags, ModuleOpen, ModuleOpenFlags,
        ModuleProvide, ModuleRequire, ModuleRequireFlags,
    },
};

use super::{
    attribute::Attribute, constant_pool::ConstantPoolEntry, parsing_context::ParsingContext,
    reader_utils::read_u16,
};

impl ModuleRequire {
    fn parse_multiple<R>(
        reader: &mut R,
        requires_count: u16,
        ctx: &ParsingContext,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        (0..requires_count)
            .map(|_| {
                let module_index = read_u16(reader)?;
                let module = ctx.constant_pool.get_module_ref(module_index)?;
                let flag_bits = read_u16(reader)?;
                let Some(flags) = ModuleRequireFlags::from_bits(flag_bits) else {
                    return Err(ClassFileParsingError::UnknownFlags(
                        flag_bits,
                        "module_require",
                    ));
                };
                let version_index = read_u16(reader)?;
                let version = if version_index > 0 {
                    Some(ctx.constant_pool.get_str(version_index)?.to_owned())
                } else {
                    None
                };
                Ok(ModuleRequire {
                    module,
                    flags,
                    version,
                })
            })
            .collect::<Result<_, ClassFileParsingError>>()
    }
}

impl ModuleExport {
    fn parse_multiple<R>(
        reader: &mut R,
        count: u16,
        ctx: &ParsingContext,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        (0..count)
            .map(|_| {
                let package_index = read_u16(reader)?;
                let package = ctx.constant_pool.get_package_ref(package_index)?;
                let flag_bits = read_u16(reader)?;
                let Some(flags) = ModuleExportFlags::from_bits(flag_bits) else {
                    return Err(ClassFileParsingError::UnknownFlags(
                        flag_bits,
                        "module_export",
                    ));
                };
                let to_count = read_u16(reader)?;
                let mut to = Vec::with_capacity(to_count as usize);
                for _ in 0..to_count {
                    let module_index = read_u16(reader)?;
                    let module = ctx.constant_pool.get_module_ref(module_index)?;
                    to.push(module);
                }
                Ok(ModuleExport { package, flags, to })
            })
            .collect::<Result<_, ClassFileParsingError>>()
    }
}

impl ModuleOpen {
    fn parse_multiple<R>(
        reader: &mut R,
        count: u16,
        ctx: &ParsingContext,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        (0..count)
            .map(|_| {
                let package_index = read_u16(reader)?;
                let package = ctx.constant_pool.get_package_ref(package_index)?;
                let flag_bits = read_u16(reader)?;
                let Some(flags) = ModuleOpenFlags::from_bits(flag_bits) else {
                    return Err(ClassFileParsingError::UnknownFlags(
                        flag_bits,
                        "module_open",
                    ));
                };
                let to_count = read_u16(reader)?;
                let mut to = Vec::with_capacity(to_count as usize);
                for _ in 0..to_count {
                    let module_index = read_u16(reader)?;
                    let module = ctx.constant_pool.get_module_ref(module_index)?;
                    to.push(module);
                }
                Ok(ModuleOpen { package, flags, to })
            })
            .collect::<Result<_, ClassFileParsingError>>()
    }
}

impl ModuleProvide {
    fn parse_multiple<R>(
        reader: &mut R,
        count: u16,
        ctx: &ParsingContext,
    ) -> Result<Vec<Self>, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        (0..count)
            .map(|_| {
                let service_index = read_u16(reader)?;
                let service = ctx.constant_pool.get_class_ref(service_index)?;
                let with_count = read_u16(reader)?;
                let mut with = Vec::with_capacity(with_count as usize);
                for _ in 0..with_count {
                    let provider_idx = read_u16(reader)?;
                    let provider = ctx.constant_pool.get_class_ref(provider_idx)?;
                    with.push(provider);
                }
                Ok(ModuleProvide { service, with })
            })
            .collect::<Result<_, ClassFileParsingError>>()
    }
}

impl Attribute {
    pub(super) fn parse_module<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let module_info_idx = read_u16(reader)?;
        let module_info_entry = ctx.constant_pool.get_entry(module_info_idx)?;
        let &ConstantPoolEntry::Module { name_index } = module_info_entry else {
            Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "Module",
                found: module_info_entry.type_name(),
            })?
        };
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let flag_bits = read_u16(reader)?;
        let Some(flags) = ModuleFlags::from_bits(flag_bits) else {
            return Err(ClassFileParsingError::UnknownFlags(flag_bits, "module"));
        };
        let version_index = read_u16(reader)?;
        let version = if version_index > 0 {
            Some(ctx.constant_pool.get_str(version_index)?.to_owned())
        } else {
            None
        };
        let requires_count = read_u16(reader)?;
        let requires = ModuleRequire::parse_multiple(reader, requires_count, ctx)?;
        let exports_count = read_u16(reader)?;
        let exports = ModuleExport::parse_multiple(reader, exports_count, ctx)?;
        let opens_count = read_u16(reader)?;
        let opens = ModuleOpen::parse_multiple(reader, opens_count, ctx)?;
        let uses_count = read_u16(reader)?;
        let mut uses = Vec::with_capacity(uses_count as usize);
        for _ in 0..uses_count {
            let class_index = read_u16(reader)?;
            let class = ctx.constant_pool.get_class_ref(class_index)?;
            uses.push(class);
        }
        let provides_count = read_u16(reader)?;
        let provides = ModuleProvide::parse_multiple(reader, provides_count, ctx)?;
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
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let package_count = read_u16(reader)?;
        let mut packages = Vec::with_capacity(package_count as usize);
        for _ in 0..package_count {
            let package_index = read_u16(reader)?;
            let package = ctx.constant_pool.get_package_ref(package_index)?;
            packages.push(package);
        }
        Ok(Self::ModulePackages(packages))
    }
    pub(super) fn parse_module_main_class<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<Self, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let main_class_index = read_u16(reader)?;
        let main_class = ctx.constant_pool.get_class_ref(main_class_index)?;
        Ok(Self::ModuleMainClass(main_class))
    }
}
