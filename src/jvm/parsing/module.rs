use crate::jvm::{
    constant_pool::Entry,
    module::{Export, Module, Open, Provide, Require},
};

use super::{jvm_element_parser::ClassElement, raw_attributes, Context, Error};

impl ClassElement for Require {
    type Raw = raw_attributes::RequiresInfo;
    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            requires_index,
            flags,
            version_index,
        } = raw;
        let module = ctx.constant_pool.get_module_ref(requires_index)?;
        let flags = ClassElement::from_raw(flags, ctx)?;
        let version = if version_index > 0 {
            Some(ctx.constant_pool.get_str(version_index)?.to_owned())
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

impl ClassElement for Export {
    type Raw = raw_attributes::ExportsInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            exports_index,
            to,
            flags,
        } = raw;

        let package = ctx.constant_pool.get_package_ref(exports_index)?;
        let flags = ClassElement::from_raw(flags, ctx)?;
        let to = to
            .into_iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx))
            .collect::<Result<_, _>>()?;
        Ok(Export { package, flags, to })
    }
}

impl ClassElement for Open {
    type Raw = raw_attributes::OpensInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            opens_index,
            to,
            flags,
        } = raw;

        let package = ctx.constant_pool.get_package_ref(opens_index)?;
        let flags = ClassElement::from_raw(flags, ctx)?;
        let to = to
            .into_iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx))
            .collect::<Result<_, _>>()?;
        Ok(Open { package, flags, to })
    }
}

impl ClassElement for Provide {
    type Raw = raw_attributes::ProvidesInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            provides_index,
            with,
        } = raw;
        let service = ctx.constant_pool.get_class_ref(provides_index)?;
        let with = with
            .into_iter()
            .map(|idx| ctx.constant_pool.get_class_ref(idx))
            .collect::<Result<_, _>>()?;
        Ok(Provide { service, with })
    }
}

impl ClassElement for Module {
    type Raw = raw_attributes::ModuleInfo;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        let Self::Raw {
            info_index,
            flags,
            version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        } = raw;
        let module_info_entry = ctx.constant_pool.get_entry(info_index)?;
        let &Entry::Module { name_index } = module_info_entry else {
            Err(Error::MismatchedConstantPoolEntryType {
                expected: "Module",
                found: module_info_entry.constant_kind(),
            })?
        };
        let name = ctx.constant_pool.get_str(name_index)?.to_owned();
        let flags = ClassElement::from_raw(flags, ctx)?;
        let version = if version_index > 0 {
            Some(ctx.constant_pool.get_str(version_index)?.to_owned())
        } else {
            None
        };
        let requires = requires
            .into_iter()
            .map(|raw| ClassElement::from_raw(raw, ctx))
            .collect::<Result<_, _>>()?;
        let exports = exports
            .into_iter()
            .map(|raw| ClassElement::from_raw(raw, ctx))
            .collect::<Result<_, _>>()?;
        let opens = opens
            .into_iter()
            .map(|raw| ClassElement::from_raw(raw, ctx))
            .collect::<Result<_, _>>()?;
        let uses = uses
            .into_iter()
            .map(|idx| ctx.constant_pool.get_class_ref(idx))
            .collect::<Result<_, _>>()?;
        let provides = provides
            .into_iter()
            .map(|raw| ClassElement::from_raw(raw, ctx))
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
