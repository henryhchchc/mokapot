//! Resolution of raw stack-map frames and verification types.

use crate::intrinsics::ExactSizeIteratorExt;

use super::raw;
use crate::jvm::{
    class_file::{ParseError, ResolutionContext, resolve::Resolve},
    code::{ProgramCounter, StackMapFrameData, VerificationTypeData},
};

impl Resolve for raw::StackMapFrameInfo {
    type Output = StackMapFrameData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        match raw {
            raw::StackMapFrameInfo::SameFrame { frame_type } => Ok(StackMapFrameData::Same {
                offset_delta: u16::from(frame_type),
            }),
            raw::StackMapFrameInfo::SameFrameExtended { offset_delta } => {
                Ok(StackMapFrameData::Same { offset_delta })
            }
            raw::StackMapFrameInfo::SameLocals1StackItemFrame { frame_type, stack } => {
                Ok(StackMapFrameData::SameLocals1StackItem {
                    offset_delta: u16::from(frame_type) - 64,
                    stack: stack.resolve(ctx)?,
                })
            }
            raw::StackMapFrameInfo::SameLocals1StackItemFrameExtended {
                offset_delta,
                stack,
            } => Ok(StackMapFrameData::SameLocals1StackItem {
                offset_delta,
                stack: stack.resolve(ctx)?,
            }),
            raw::StackMapFrameInfo::ChopFrame {
                frame_type,
                offset_delta,
            } => {
                let chop_count = 251 - frame_type;
                Ok(StackMapFrameData::Chop {
                    chop_count,
                    offset_delta,
                })
            }
            raw::StackMapFrameInfo::AppendFrame {
                offset_delta,
                locals,
            } => Ok(StackMapFrameData::Append {
                offset_delta,
                locals: locals
                    .into_iter()
                    .map(|it| it.resolve(ctx))
                    .try_collect_exact()?,
            }),
            raw::StackMapFrameInfo::FullFrame {
                offset_delta,
                locals,
                stack,
            } => {
                let locals = locals
                    .into_iter()
                    .map(|it| it.resolve(ctx))
                    .try_collect_exact()?;
                let stack = stack
                    .into_iter()
                    .map(|it| it.resolve(ctx))
                    .try_collect_exact()?;
                Ok(StackMapFrameData::Full {
                    offset_delta,
                    locals,
                    stack,
                })
            }
        }
    }
}

impl Resolve for raw::VerificationTypeInfo {
    type Output = VerificationTypeData;
    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        match raw {
            raw::VerificationTypeInfo::Top => Ok(VerificationTypeData::Top),
            raw::VerificationTypeInfo::Integer => Ok(VerificationTypeData::Integer),
            raw::VerificationTypeInfo::Float => Ok(VerificationTypeData::Float),
            raw::VerificationTypeInfo::Double => Ok(VerificationTypeData::Double),
            raw::VerificationTypeInfo::Long => Ok(VerificationTypeData::Long),
            raw::VerificationTypeInfo::Null => Ok(VerificationTypeData::Null),
            raw::VerificationTypeInfo::UninitializedThis => {
                Ok(VerificationTypeData::UninitializedThis)
            }
            raw::VerificationTypeInfo::Object { cpool_index } => Ok(VerificationTypeData::Object(
                ctx.constant_pool.reference_type_id(cpool_index)?,
            )),
            raw::VerificationTypeInfo::Uninitialized { offset } => {
                Ok(VerificationTypeData::Uninitialized {
                    offset: ProgramCounter::from(offset),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
