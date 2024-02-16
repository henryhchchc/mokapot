use crate::jvm::{
    constant_pool::Entry,
    module::{
        Module, ModuleExport, ModuleOpen, ModuleProvide, ModuleReference, ModuleRequire,
        PackageReference,
    },
};

use super::{
    jvm_element_parser::{parse_flags, parse_jvm, ParseJvmElement},
    parsing_context::ParsingContext,
    reader_utils::ClassReader,
    Error,
};

fn parse_version<R>(reader: &mut R, ctx: &ParsingContext) -> Result<Option<String>, Error>
where
    R: std::io::Read,
{
    let version_index = reader.read_value()?;
    let result = if version_index > 0 {
        Some(ctx.constant_pool.get_str(version_index)?.to_owned())
    } else {
        None
    };
    Ok(result)
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleRequire {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let module = parse_jvm!(reader, ctx)?;
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
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package = parse_jvm!(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let to = parse_jvm!(u16, reader, ctx)?;
        Ok(ModuleExport { package, flags, to })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleOpen {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package = parse_jvm!(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let to = parse_jvm!(u16, reader, ctx)?;
        Ok(ModuleOpen { package, flags, to })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleProvide {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let service = parse_jvm!(reader, ctx)?;
        let with = parse_jvm!(u16, reader, ctx)?;
        Ok(ModuleProvide { service, with })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for Module {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let module_info_idx = reader.read_value()?;
        let module_info_entry = ctx.constant_pool.get_entry(module_info_idx)?;
        let &Entry::Module { name_index } = module_info_entry else {
            Err(Error::MismatchedConstantPoolEntryType {
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
            requires: parse_jvm!(u16, reader, ctx)?,
            exports: parse_jvm!(u16, reader, ctx)?,
            opens: parse_jvm!(u16, reader, ctx)?,
            uses: parse_jvm!(u16, reader, ctx)?,
            provides: parse_jvm!(u16, reader, ctx)?,
        })
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for PackageReference {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package_index = reader.read_value()?;
        ctx.constant_pool.get_package_ref(package_index)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for ModuleReference {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let module_ref_idx = reader.read_value()?;
        ctx.constant_pool.get_module_ref(module_ref_idx)
    }
}
