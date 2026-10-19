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
            module_name_index,
            module_flags,
            module_version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        } = self;

        let name = ctx.constant_pool.module_id(module_name_index)?;
        let flags = parse_flags(module_flags, "Invalid access flag")?;
        let version = if module_version_index > 0 {
            Some(ctx.constant_pool.utf8_id(module_version_index)?)
        } else {
            None
        };

        let requires = requires
            .iter()
            .map(|raw| {
                Ok(RequireData {
                    module: ctx.constant_pool.module_id(raw.requires_index.get())?,
                    flags: parse_flags(raw.requires_flags.get(), "Invalid access flag")?,
                    version: if raw.requires_version_index.get() > 0 {
                        Some(
                            ctx.constant_pool
                                .utf8_id(raw.requires_version_index.get())?,
                        )
                    } else {
                        None
                    },
                })
            })
            .try_collect_exact()?;
        let exports = exports
            .iter()
            .map(|raw| {
                let package = ctx.constant_pool.package_id(raw.exports_index.get())?;
                let flags = parse_flags(raw.exports_flags.get(), "Invalid access flag")?;
                let to = raw
                    .exports_to
                    .iter()
                    .map(|id| ctx.constant_pool.module_id(id.get()))
                    .try_collect_exact()?;
                Ok(ExportData { package, flags, to })
            })
            .try_collect_exact()?;
        let opens = opens
            .iter()
            .map(|raw| {
                let package = ctx.constant_pool.package_id(raw.opens_index.get())?;
                let flags = parse_flags(raw.opens_flags.get(), "Invalid access flag")?;
                let to = raw
                    .opens_to
                    .iter()
                    .map(|id| ctx.constant_pool.module_id(id.get()))
                    .try_collect_exact()?;
                Ok(OpenData { package, flags, to })
            })
            .try_collect_exact()?;
        let uses = uses
            .iter()
            .map(|id| ctx.constant_pool.class_id(id.get()))
            .try_collect_exact()?;
        let provides = provides
            .iter()
            .map(|raw| {
                let service = ctx.constant_pool.class_id(raw.provides_index.get())?;
                let with = raw
                    .provides_with
                    .iter()
                    .map(|id| ctx.constant_pool.class_id(id.get()))
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
