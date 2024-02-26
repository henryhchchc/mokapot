use std::io::Read;

use crate::jvm::{
    constant_pool::Entry,
    module::{
        Module, ModuleExport, ModuleOpen, ModuleProvide, ModuleRef, ModuleRequire, PackageRef,
    },
};

use super::{
    jvm_element_parser::{parse_flags, JvmElement},
    parsing_context::ParsingContext,
    reader_utils::ValueReaderExt,
    Error,
};

fn parse_version<R: Read + ?Sized>(
    reader: &mut R,
    ctx: &ParsingContext,
) -> Result<Option<String>, Error> {
    let version_index = reader.read_value()?;
    let result = if version_index > 0 {
        Some(ctx.constant_pool.get_str(version_index)?.to_owned())
    } else {
        None
    };
    Ok(result)
}

impl JvmElement for ModuleRequire {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let module = JvmElement::parse(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let version = parse_version(reader, ctx)?;
        Ok(ModuleRequire {
            module,
            flags,
            version,
        })
    }
}

impl JvmElement for ModuleExport {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package = JvmElement::parse(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let to = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        Ok(ModuleExport { package, flags, to })
    }
}

impl JvmElement for ModuleOpen {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package = JvmElement::parse(reader, ctx)?;
        let flags = parse_flags(reader)?;
        let to = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        Ok(ModuleOpen { package, flags, to })
    }
}

impl JvmElement for ModuleProvide {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let service = JvmElement::parse(reader, ctx)?;
        let with = JvmElement::parse_vec::<u16, _>(reader, ctx)?;
        Ok(ModuleProvide { service, with })
    }
}

impl JvmElement for Module {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
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
            requires: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
            exports: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
            opens: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
            uses: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
            provides: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
        })
    }
}

impl JvmElement for PackageRef {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let package_index = reader.read_value()?;
        ctx.constant_pool.get_package_ref(package_index)
    }
}

impl JvmElement for ModuleRef {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let module_ref_idx = reader.read_value()?;
        ctx.constant_pool.get_module_ref(module_ref_idx)
    }
}
