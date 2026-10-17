//! Resolution of raw stack-map frames and verification types.

use crate::intrinsics::ExactSizeIteratorExt;

use super::raw;
use crate::jvm::{
    class_file::{ParseError, ResolutionContext, resolve::Resolve},
    code::{ProgramCounter, StackMapFrame, VerificationType},
};

impl Resolve for raw::StackMapFrameInfo {
    type Output = StackMapFrame;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        match raw {
            raw::StackMapFrameInfo::SameFrame { frame_type } => Ok(StackMapFrame::SameFrame {
                offset_delta: u16::from(frame_type),
            }),
            raw::StackMapFrameInfo::SameFrameExtended { offset_delta } => {
                Ok(StackMapFrame::SameFrame { offset_delta })
            }
            raw::StackMapFrameInfo::SameLocals1StackItemFrame { frame_type, stack } => {
                Ok(StackMapFrame::SameLocals1StackItemFrame {
                    offset_delta: u16::from(frame_type) - 64,
                    stack: stack.resolve(ctx)?,
                })
            }
            raw::StackMapFrameInfo::SameLocals1StackItemFrameExtended {
                offset_delta,
                stack,
            } => Ok(StackMapFrame::SameLocals1StackItemFrame {
                offset_delta,
                stack: stack.resolve(ctx)?,
            }),
            raw::StackMapFrameInfo::ChopFrame {
                frame_type,
                offset_delta,
            } => {
                let chop_count = 251 - frame_type;
                Ok(StackMapFrame::ChopFrame {
                    chop_count,
                    offset_delta,
                })
            }
            raw::StackMapFrameInfo::AppendFrame {
                offset_delta,
                locals,
            } => Ok(StackMapFrame::AppendFrame {
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
                Ok(StackMapFrame::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                })
            }
        }
    }
}

impl Resolve for raw::VerificationTypeInfo {
    type Output = VerificationType;
    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        match raw {
            raw::VerificationTypeInfo::Top => Ok(VerificationType::TopVariable),
            raw::VerificationTypeInfo::Integer => Ok(VerificationType::IntegerVariable),
            raw::VerificationTypeInfo::Float => Ok(VerificationType::FloatVariable),
            raw::VerificationTypeInfo::Double => Ok(VerificationType::DoubleVariable),
            raw::VerificationTypeInfo::Long => Ok(VerificationType::LongVariable),
            raw::VerificationTypeInfo::Null => Ok(VerificationType::NullVariable),
            raw::VerificationTypeInfo::UninitializedThis => {
                Ok(VerificationType::UninitializedThisVariable)
            }
            raw::VerificationTypeInfo::Object { cpool_index } => Ok(
                VerificationType::ObjectVariable(ctx.constant_pool.get_type_ref(cpool_index)?),
            ),
            raw::VerificationTypeInfo::Uninitialized { offset } => {
                Ok(VerificationType::UninitializedVariable {
                    offset: ProgramCounter::from(offset),
                })
            }
        }
    }
}

#[cfg(test)]
mod tests;
