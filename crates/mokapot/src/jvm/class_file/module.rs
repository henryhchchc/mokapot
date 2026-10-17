//! Module decoding and resolution.

pub(super) mod raw;

use crate::jvm::class_file::flags::parse_flags;

use super::{ParseError, ResolutionContext, resolve::Resolve};
use crate::jvm::{
    Module,
    module::{Export, Open, Provide, Require},
};

impl Resolve for raw::RequiresInfo {
    type Output = Require;
    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::RequiresInfo {
            requires_index,
            requires_flags,
            requires_version_index,
        } = raw;
        let requires_index = requires_index.get();
        let requires_flags = requires_flags.get();
        let requires_version_index = requires_version_index.get();
        let module = ctx.constant_pool.get_module_ref(requires_index)?;
        let flags = parse_flags(requires_flags, "Invalid access flag")?;
        let version = if requires_version_index > 0 {
            Some(
                ctx.constant_pool
                    .get_str(requires_version_index)?
                    .to_owned(),
            )
        } else {
            None
        };
        Ok(Require {
            module,
            flags,
            version,
        })
    }
}

impl Resolve for raw::ExportsInfo<'_> {
    type Output = Export;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::ExportsInfo {
            exports_index,
            exports_to,
            exports_flags,
        } = raw;

        let package = ctx.constant_pool.get_package_name(exports_index)?;
        let flags = parse_flags(exports_flags, "Invalid access flag")?;
        let to = exports_to
            .iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx.get()))
            .collect::<Result<_, _>>()?;
        Ok(Export { package, flags, to })
    }
}

impl Resolve for raw::OpensInfo<'_> {
    type Output = Open;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::OpensInfo {
            opens_index,
            opens_to,
            opens_flags,
        } = raw;

        let package = ctx.constant_pool.get_package_name(opens_index)?;
        let flags = parse_flags(opens_flags, "Invalid access flag")?;
        let to = opens_to
            .iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx.get()))
            .collect::<Result<_, _>>()?;
        Ok(Open { package, flags, to })
    }
}

impl Resolve for raw::ProvidesInfo<'_> {
    type Output = Provide;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::ProvidesInfo {
            provides_index,
            provides_with,
        } = raw;
        let service = ctx.constant_pool.get_class_name(provides_index)?;
        let with = provides_with
            .iter()
            .map(|idx| ctx.constant_pool.get_class_name(idx.get()))
            .collect::<Result<_, _>>()?;
        Ok(Provide { service, with })
    }
}

impl Resolve for raw::ModuleInfo<'_> {
    type Output = Module;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::ModuleInfo {
            module_name_index,
            module_flags,
            module_version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        } = raw;
        let name = ctx.constant_pool.get_module_ref(module_name_index)?.name;
        let flags = parse_flags(module_flags, "Invalid access flag")?;
        let version = if module_version_index > 0 {
            Some(ctx.constant_pool.get_str(module_version_index)?.to_owned())
        } else {
            None
        };
        let requires = requires
            .iter()
            .copied()
            .map(|raw| raw.resolve(ctx))
            .collect::<Result<_, _>>()?;
        let exports = exports
            .into_iter()
            .map(|raw| raw.resolve(ctx))
            .collect::<Result<_, _>>()?;
        let opens = opens
            .into_iter()
            .map(|raw| raw.resolve(ctx))
            .collect::<Result<_, _>>()?;
        let uses = uses
            .iter()
            .map(|idx| ctx.constant_pool.get_class_name(idx.get()))
            .collect::<Result<_, _>>()?;
        let provides = provides
            .into_iter()
            .map(|raw| raw.resolve(ctx))
            .collect::<Result<_, _>>()?;
        Ok(Module {
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
