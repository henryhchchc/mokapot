//! Resolution of raw attribute payloads.

use zerocopy::byteorder::big_endian::U16;

use super::super::{
    ParseError, ResolutionContext,
    code::{LocalVariableDescriptorEntry, LocalVariableSignatureEntry},
    flags::parse_flags,
    resolve::{Resolve, resolve_table},
};
use super::raw;
use crate::{
    intrinsics::ExactSizeIteratorExt,
    jvm::{
        ConstantValue, JavaString,
        class::InnerClassInfo,
        code::LineNumberTableEntry,
        method::{ParameterAccessFlags, ParameterInfo},
    },
    types::{class_name::ClassName, package_name::PackageName},
};

impl Resolve for raw::ConstantValue {
    type Output = ConstantValue;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .get_constant_value(self.constantvalue_index.get())
    }
}

impl Resolve for raw::Signature {
    type Output = JavaString;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .get_java_string(self.signature_index.get())
    }
}

impl Resolve for raw::SourceFile {
    type Output = String;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .get_str(self.sourcefile_index.get())
            .map(std::borrow::Cow::into_owned)
    }
}

impl Resolve for raw::ModuleMainClass {
    type Output = ClassName;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .get_class_name(self.main_class_index.get())
    }
}

impl Resolve for raw::NestHost {
    type Output = ClassName;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .get_class_name(self.host_class_index.get())
    }
}

impl Resolve for &raw::Exceptions {
    type Output = Box<[ClassName]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_indices(&self.exception_index_table, |index| {
            ctx.constant_pool.get_class_name(index)
        })
    }
}

impl Resolve for &raw::InnerClasses {
    type Output = Box<[InnerClassInfo]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.classes, ctx)
    }
}

impl Resolve for &raw::LineNumberTable {
    type Output = Box<[LineNumberTableEntry]>;

    fn resolve(self, _ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        Ok(self
            .line_number_table
            .iter()
            .copied()
            .map(Into::into)
            .collect())
    }
}

impl Resolve for &raw::LocalVariableTable {
    type Output = Box<[LocalVariableDescriptorEntry]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.local_variable_table, ctx)
    }
}

impl Resolve for &raw::LocalVariableTypeTable {
    type Output = Box<[LocalVariableSignatureEntry]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.local_variable_type_table, ctx)
    }
}

impl Resolve for &raw::MethodParameters {
    type Output = Box<[ParameterInfo]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.parameters, ctx)
    }
}

impl Resolve for &raw::ModulePackages {
    type Output = Box<[PackageName]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_indices(&self.package_index, |index| {
            ctx.constant_pool.get_package_name(index)
        })
    }
}

impl Resolve for &raw::NestMembers {
    type Output = Box<[ClassName]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_indices(&self.classes, |index| {
            ctx.constant_pool.get_class_name(index)
        })
    }
}

impl Resolve for &raw::PermittedSubclasses {
    type Output = Box<[ClassName]>;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_indices(&self.classes, |index| {
            ctx.constant_pool.get_class_name(index)
        })
    }
}

fn resolve_indices<T>(
    indices: &[U16],
    mut resolve: impl FnMut(u16) -> Result<T, ParseError>,
) -> Result<Box<[T]>, ParseError> {
    indices
        .iter()
        .map(|index| resolve(index.get()))
        .try_collect_exact()
}

impl Resolve for raw::MethodParameterInfo {
    type Output = ParameterInfo;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::MethodParameterInfo {
            name_index,
            access_flags,
        } = raw;
        let name_index = name_index.get();
        let access_flags = access_flags.get();
        let name = if name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(name_index)?.into_owned())
        };
        let access_flags =
            parse_flags::<ParameterAccessFlags>(access_flags, "Invalid parameter access flags")?;
        Ok(ParameterInfo { name, access_flags })
    }
}
