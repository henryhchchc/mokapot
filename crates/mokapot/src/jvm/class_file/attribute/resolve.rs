//! Resolution of raw attribute payloads.

use super::super::{
    Decode, ParseError, ResolutionContext,
    code::{LocalVariableDescriptorEntry, LocalVariableSignatureEntry},
    flags::parse_flags,
    resolve::{Resolve, resolve_table},
};
use super::raw;
use crate::{
    intrinsics::ExactSizeIteratorExt,
    jvm::{
        ConstantPoolIndex,
        class::nesting::InnerClassData,
        code::LineNumberTableEntry,
        method::{ParameterAccessFlags, ParameterInfoData},
    },
};

pub(super) fn decode_pool_indices<T>(
    bytes: &mut &[u8],
    resolve: impl FnMut(ConstantPoolIndex) -> Result<T, ParseError>,
) -> Result<Box<[T]>, ParseError> {
    <&raw::ConstantPoolIndices>::decode(bytes)?
        .indices
        .iter()
        .copied()
        .map(resolve)
        .try_collect_exact()
}

impl Resolve for &raw::InnerClasses {
    type Output = Box<[InnerClassData]>;

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

impl Resolve for raw::MethodParameterInfo {
    type Output = ParameterInfoData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::MethodParameterInfo { name, access_flags } = raw;
        let access_flags = access_flags.get();
        let name = name
            .get()
            .map(|index| ctx.constant_pool.utf8_id(index))
            .transpose()?;
        let access_flags =
            parse_flags::<ParameterAccessFlags>(access_flags, "Invalid parameter access flags")?;
        Ok(ParameterInfoData { name, access_flags })
    }
}
