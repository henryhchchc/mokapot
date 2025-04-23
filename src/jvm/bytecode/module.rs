use itertools::Itertools;

use super::{
    GenerationError, ParsingContext, ParseError, errors::ParsingErrorContext,
    jvm_element_parser::ClassElement, raw_attributes,
};
use crate::jvm::{
    Module,
    class::{ConstantPool, constant_pool::Entry},
    module::{Export, Open, Provide, Require},
};

impl ClassElement for Require {
    type Raw = raw_attributes::RequiresInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let requires_index = cp.put_module_ref(self.module)?;
        let flags = self.flags.into_raw(cp)?;
        let version_index = self
            .version
            .map(|it| cp.put_string(it))
            .transpose()?
            .unwrap_or(0);
        Ok(Self::Raw {
            requires_index,
            flags,
            version_index,
        })
    }
}

impl ClassElement for Export {
    type Raw = raw_attributes::ExportsInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let exports_index = cp.put_package_ref(self.package)?;
        let flags = self.flags.into_raw(cp)?;
        let to = self
            .to
            .into_iter()
            .map(|it| cp.put_module_ref(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            exports_index,
            flags,
            to,
        })
    }
}

impl ClassElement for Open {
    type Raw = raw_attributes::OpensInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let opens_index = cp.put_package_ref(self.package)?;
        let flags = self.flags.into_raw(cp)?;
        let to = self
            .to
            .into_iter()
            .map(|it| cp.put_module_ref(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            opens_index,
            flags,
            to,
        })
    }
}

impl ClassElement for Provide {
    type Raw = raw_attributes::ProvidesInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let provides_index = cp.put_class_ref(self.service)?;
        let with = self
            .with
            .into_iter()
            .map(|it| cp.put_class_ref(it))
            .collect::<Result<_, _>>()?;
        Ok(Self::Raw {
            provides_index,
            with,
        })
    }
}

impl ClassElement for Module {
    type Raw = raw_attributes::ModuleInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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
        let module_info_entry = ctx
            .constant_pool
            .get_entry(info_index)
            .context("Invalid constant pool index")?;
        let &Entry::Module { name_index } = module_info_entry else {
            Err(ParseError::malform(format!(
                "Mismatched constant pool type. Expected Module, but got {}.",
                module_info_entry.constant_kind()
            )))?
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let name_index = cp.put_string(self.name)?;
        let info_index = cp.put_entry_dedup(Entry::Module { name_index })?;
        let flags = self.flags.into_raw(cp)?;
        let version_index = self
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
            .into_iter()
            .map(|it| cp.put_class_ref(it))
            .try_collect()?;
        Ok(Self::Raw {
            info_index,
            flags,
            version_index,
            requires,
            exports,
            opens,
            uses,
            provides,
        })
    }
}
