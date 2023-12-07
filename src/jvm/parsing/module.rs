use crate::{
    jvm::ClassFileParsingError,
    jvm::{
        module::{
            Module, ModuleExport, ModuleOpen, ModuleProvide, ModuleReference, ModuleRequire,
            PackageReference,
        },
        ClassFileParsingResult,
    },
};

use super::{
    constant_pool::ConstantPoolEntry,
    jvm_element_parser::{parse_flags, parse_jvm_element, ParseJvmElement},
    parsing_context::ParsingContext,
    reader_utils::read_u16,
};

fn parse_version<R>(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Option<String>>
where
    R: std::io::Read,
{
    let version_index = read_u16(reader)?;
    let result = if version_index > 0 {
        Some(ctx.constant_pool.get_str(version_index)?.to_owned())
    } else {
        None
    };
    Ok(result)
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleRequire {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let module = parse_jvm_element(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let version = parse_version(reader, ctx)?;
        Ok(ModuleRequire {
            module,
            flags,
            version,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleExport {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let package_index = read_u16(reader)?;
        let package = ctx.constant_pool.get_package_ref(package_index)?;
        let flags = parse_flags(reader)?;
        let to = parse_jvm_element(reader, ctx)?;
        Ok(ModuleExport { package, flags, to })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleOpen {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let package = parse_jvm_element(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let to = parse_jvm_element(reader, ctx)?;
        Ok(ModuleOpen { package, flags, to })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleProvide {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let service_index = read_u16(reader)?;
        let service = ctx.constant_pool.get_class_ref(service_index)?;
        let with = parse_jvm_element(reader, ctx)?;
        Ok(ModuleProvide { service, with })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for Module {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let module_info_idx = read_u16(reader)?;
        let module_info_entry = ctx.constant_pool.get_entry_internal(module_info_idx)?;
        let &ConstantPoolEntry::Module { name_index } = module_info_entry else {
            Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                expected: "Module",
                found: module_info_entry.constant_kind(),
            })?
        };
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let flags = parse_flags(reader)?;
        let version = parse_version(reader, ctx)?;
        Ok(Module {
            name,
            flags,
            version,
            requires: parse_jvm_element(reader, ctx)?,
            exports: parse_jvm_element(reader, ctx)?,
            opens: parse_jvm_element(reader, ctx)?,
            uses: parse_jvm_element(reader, ctx)?,
            provides: parse_jvm_element(reader, ctx)?,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for PackageReference {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let package_index = read_u16(reader)?;
        ctx.constant_pool.get_package_ref(package_index)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleReference {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> ClassFileParsingResult<Self> {
        let module_ref_idx = read_u16(reader)?;
        ctx.constant_pool.get_module_ref(module_ref_idx)
    }
}
