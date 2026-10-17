use crate::jvm::{
    bytecode::{ParseError, ParsingContext, class_element::ClassElement, raw_attributes},
    code::{ProgramCounter, StackMapFrame, VerificationType},
};

impl ClassElement<'_> for StackMapFrame {
    type Raw = raw_attributes::StackMapFrameInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        match raw {
            Self::Raw::SameFrame { frame_type } => Ok(Self::SameFrame {
                offset_delta: u16::from(frame_type),
            }),
            Self::Raw::SameFrameExtended { offset_delta } => Ok(Self::SameFrame { offset_delta }),
            Self::Raw::SameLocals1StackItemFrame { frame_type, stack } => {
                Ok(Self::SameLocals1StackItemFrame {
                    offset_delta: u16::from(frame_type) - 64,
                    stack: ClassElement::from_raw(stack, ctx)?,
                })
            }
            Self::Raw::SameLocals1StackItemFrameExtended {
                offset_delta,
                stack,
            } => Ok(Self::SameLocals1StackItemFrame {
                offset_delta,
                stack: ClassElement::from_raw(stack, ctx)?,
            }),
            Self::Raw::ChopFrame {
                frame_type,
                offset_delta,
            } => {
                let chop_count = 251 - frame_type;
                Ok(Self::ChopFrame {
                    chop_count,
                    offset_delta,
                })
            }
            Self::Raw::AppendFrame {
                offset_delta,
                locals,
            } => Ok(Self::AppendFrame {
                offset_delta,
                locals: locals
                    .into_iter()
                    .map(|it| ClassElement::from_raw(it, ctx))
                    .collect::<Result<_, _>>()?,
            }),
            Self::Raw::FullFrame {
                offset_delta,
                locals,
                stack,
            } => {
                let locals = locals
                    .into_iter()
                    .map(|it| ClassElement::from_raw(it, ctx))
                    .collect::<Result<_, _>>()?;
                let stack = stack
                    .into_iter()
                    .map(|it| ClassElement::from_raw(it, ctx))
                    .collect::<Result<_, _>>()?;
                Ok(Self::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                })
            }
        }
    }
}

impl ClassElement<'_> for VerificationType {
    type Raw = raw_attributes::VerificationTypeInfo;
    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        match raw {
            Self::Raw::Top => Ok(Self::TopVariable),
            Self::Raw::Integer => Ok(Self::IntegerVariable),
            Self::Raw::Float => Ok(Self::FloatVariable),
            Self::Raw::Double => Ok(Self::DoubleVariable),
            Self::Raw::Long => Ok(Self::LongVariable),
            Self::Raw::Null => Ok(Self::NullVariable),
            Self::Raw::UninitializedThis => Ok(Self::UninitializedThisVariable),
            Self::Raw::Object { cpool_index } => Ok(Self::ObjectVariable(
                ctx.constant_pool.get_type_ref(cpool_index)?,
            )),
            Self::Raw::Uninitialized { offset } => Ok(Self::UninitializedVariable {
                offset: ProgramCounter::from(offset),
            }),
        }
    }
}
