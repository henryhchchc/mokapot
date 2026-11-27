//! Module attribute decoding and resolution.

use crate::intrinsics::ExactSizeIteratorExt;

pub(super) mod raw;

use crate::jvm::class_file::flags::parse_flags;
use crate::jvm::{
    class_file::{ParseError, ResolutionContext, resolve::Resolve},
    module::{ExportData, ModuleData, OpenData, ProvideData, RequireData},
};

impl Resolve for raw::ModuleInfo<'_> {
    type Output = ModuleData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw::ModuleInfo {
            module_name,
            module_flags,
            module_version,
            requires,
            exports,
            opens,
            uses,
            provides,
        } = self;

        let name = ctx.constant_pool.module_id(module_name)?;
        let flags = parse_flags(module_flags, "Invalid access flag")?;
        let version = module_version
            .map(|index| ctx.constant_pool.utf8_id(index))
            .transpose()?;

        let requires = requires
            .iter()
            .map(|raw| {
                Ok(RequireData {
                    module: ctx.constant_pool.module_id(raw.module)?,
                    flags: parse_flags(raw.requires_flags.get(), "Invalid access flag")?,
                    version: raw
                        .requires_version
                        .get()
                        .map(|index| ctx.constant_pool.utf8_id(index))
                        .transpose()?,
                })
            })
            .try_collect_exact()?;
        let exports = exports
            .iter()
            .map(|raw| {
                let package = ctx.constant_pool.package_id(raw.package)?;
                let flags = parse_flags(raw.exports_flags.get(), "Invalid access flag")?;
                let to = raw
                    .exports_to
                    .iter()
                    .map(|&id| ctx.constant_pool.module_id(id))
                    .try_collect_exact()?;
                Ok(ExportData { package, flags, to })
            })
            .try_collect_exact()?;
        let opens = opens
            .iter()
            .map(|raw| {
                let package = ctx.constant_pool.package_id(raw.package)?;
                let flags = parse_flags(raw.opens_flags.get(), "Invalid access flag")?;
                let to = raw
                    .opens_to
                    .iter()
                    .map(|&id| ctx.constant_pool.module_id(id))
                    .try_collect_exact()?;
                Ok(OpenData { package, flags, to })
            })
            .try_collect_exact()?;
        let uses = uses
            .iter()
            .map(|&id| ctx.constant_pool.class_id(id))
            .try_collect_exact()?;
        let provides = provides
            .iter()
            .map(|raw| {
                let service = ctx.constant_pool.class_id(raw.service)?;
                let with = raw
                    .provides_with
                    .iter()
                    .map(|&id| ctx.constant_pool.class_id(id))
                    .try_collect_exact()?;
                Ok(ProvideData { service, with })
            })
            .try_collect_exact()?;

        Ok(ModuleData {
            name,
            flags,
            version,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}
