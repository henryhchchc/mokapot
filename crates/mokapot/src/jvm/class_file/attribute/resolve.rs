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
        code::LineNumberTableEntry,
        method::{ParameterAccessFlags, ParameterInfoData},
    },
};

impl Resolve for raw::ConstantValue {
    type Output = super::super::constant_pool::ConstantValueId;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        ctx.constant_pool
            .intern_constant(self.constantvalue_index.get())
    }
}

impl Resolve for &raw::Exceptions {
    type Output = Box<[super::super::constant_pool::ClassId]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_indices(&self.exception_index_table, |index| {
            ctx.constant_pool.class_id(index)
        })
    }
}

impl Resolve for &raw::InnerClasses {
    type Output = Box<[crate::jvm::class::nesting::InnerClassData]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.classes, ctx)
    }
}

impl Resolve for &raw::LineNumberTable {
    type Output = Box<[LineNumberTableEntry]>;

    fn resolve(self, _ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
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

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.local_variable_table, ctx)
    }
}

impl Resolve for &raw::LocalVariableTypeTable {
    type Output = Box<[LocalVariableSignatureEntry]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.local_variable_type_table, ctx)
    }
}

impl Resolve for &raw::MethodParameters {
    type Output = Box<[ParameterInfoData]>;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        resolve_table(&self.parameters, ctx)
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
    type Output = ParameterInfoData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
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
            Some(ctx.constant_pool.utf8_id(name_index)?)
        };
        let access_flags =
            parse_flags::<ParameterAccessFlags>(access_flags, "Invalid parameter access flags")?;
        Ok(ParameterInfoData { name, access_flags })
    }
}
