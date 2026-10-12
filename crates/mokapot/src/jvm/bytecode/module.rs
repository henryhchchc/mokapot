//! [`ClassElement`] implementations for module types.

use itertools::Itertools;

use super::{
    GenerationError, ParseError, ParsingContext, class_element::ClassElement,
    constant_pool::RawConstantPool, raw_attributes,
};
use crate::jvm::{
    Module,
    module::{Export, Open, Provide, Require},
    references::ModuleRef,
};

impl ClassElement for Require {
    type Raw = raw_attributes::RequiresInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            requires_index,
            requires_flags,
            requires_version_index,
        } = raw;
        let requires_index = requires_index.get();
        let requires_flags = requires_flags.get();
        let requires_version_index = requires_version_index.get();
        let module = ctx.constant_pool.get_module_ref(requires_index)?;
        let flags = ClassElement::from_raw(requires_flags, ctx)?;
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

    fn into_raw(self, cp: &mut RawConstantPool) -> Result<Self::Raw, GenerationError> {
        let requires_index = cp.put_module_ref(self.module)?.into();
        let requires_flags = self.flags.into_raw(cp)?.into();
        let requires_version_index = self
            .version
            .map(|it| cp.put_string(it))
            .transpose()?
            .unwrap_or(0)
            .into();
        Ok(Self::Raw {
            requires_index,
            requires_flags,
            requires_version_index,
        })
    }
}

impl ClassElement for Export {
    type Raw = raw_attributes::ExportsInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            exports_index,
            exports_to,
            exports_flags,
        } = raw;

        let package = ctx.constant_pool.get_package_name(exports_index)?;
        let flags = ClassElement::from_raw(exports_flags, ctx)?;
        let to = exports_to
            .into_iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx))
            .collect::<Result<_, _>>()?;
        Ok(Export { package, flags, to })
    }

    fn into_raw(self, cp: &mut RawConstantPool) -> Result<Self::Raw, GenerationError> {
        let exports_index = cp.put_package_name(&self.package)?;
        let exports_flags = self.flags.into_raw(cp)?;
        let exports_to = self
            .to
            .into_iter()
            .map(|it| cp.put_module_ref(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            exports_index,
            exports_flags,
            exports_to,
        })
    }
}

impl ClassElement for Open {
    type Raw = raw_attributes::OpensInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            opens_index,
            opens_to,
            opens_flags,
        } = raw;

        let package = ctx.constant_pool.get_package_name(opens_index)?;
        let flags = ClassElement::from_raw(opens_flags, ctx)?;
        let to = opens_to
            .into_iter()
            .map(|idx| ctx.constant_pool.get_module_ref(idx))
            .collect::<Result<_, _>>()?;
        Ok(Open { package, flags, to })
    }

    fn into_raw(self, cp: &mut RawConstantPool) -> Result<Self::Raw, GenerationError> {
        let opens_index = cp.put_package_name(&self.package)?;
        let opens_flags = self.flags.into_raw(cp)?;
        let opens_to = self
            .to
            .into_iter()
            .map(|it| cp.put_module_ref(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            opens_index,
            opens_flags,
            opens_to,
        })
    }
}

impl ClassElement for Provide {
    type Raw = raw_attributes::ProvidesInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
            provides_index,
            provides_with,
        } = raw;
        let service = ctx.constant_pool.get_class_name(provides_index)?;
        let with = provides_with
            .into_iter()
            .map(|idx| ctx.constant_pool.get_class_name(idx))
            .collect::<Result<_, _>>()?;
        Ok(Provide { service, with })
    }

    fn into_raw(self, cp: &mut RawConstantPool) -> Result<Self::Raw, GenerationError> {
        let provides_index = cp.put_class_name(&self.service)?;
        let provides_with = self
            .with
            .iter()
            .map(|it| cp.put_class_name(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            provides_index,
            provides_with,
        })
    }
}

impl ClassElement for Module {
    type Raw = raw_attributes::ModuleInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let Self::Raw {
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
        let flags = ClassElement::from_raw(module_flags, ctx)?;
        let version = if module_version_index > 0 {
            Some(ctx.constant_pool.get_str(module_version_index)?.to_owned())
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
            .map(|idx| ctx.constant_pool.get_class_name(idx))
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

    fn into_raw(self, cp: &mut RawConstantPool) -> Result<Self::Raw, GenerationError> {
        let module_name_index = cp.put_module_ref(ModuleRef { name: self.name })?;
        let module_flags = self.flags.into_raw(cp)?;
        let module_version_index = self
            .version
            .map(|it| cp.put_string(it))
            .transpose()?
            .unwrap_or(0);
        let requires = self
            .requires
            .into_iter()
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        let exports = self
            .exports
            .into_iter()
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        let opens = self
            .opens
            .into_iter()
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        let provides = self
            .provides
            .into_iter()
            .map(|it| it.into_raw(cp))
            .try_collect()?;
        let uses = self
            .uses
            .iter()
            .map(|it| cp.put_class_name(it))
            .try_collect()?;
        Ok(Self::Raw {
            module_name_index,
            module_flags,
            module_version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}
