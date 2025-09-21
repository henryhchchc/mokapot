use itertools::Itertools;

use crate::jvm::{
    bytecode::{
        ParseError, ParsingContext, errors::GenerationError, jvm_element_parser::ClassElement,
        raw_attributes,
    },
    class::ConstantPool,
    code::{ProgramCounter, StackMapFrame, VerificationType},
};

impl ClassElement for StackMapFrame {
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

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        let raw = match self {
            Self::SameFrame { offset_delta } => {
                if offset_delta < 64 {
                    let frame_type = u8::try_from(offset_delta).expect("0 - 63 should fit in u8");
                    Self::Raw::SameFrame { frame_type }
                } else {
                    Self::Raw::SameFrameExtended { offset_delta }
                }
            }
            Self::SameLocals1StackItemFrame {
                offset_delta,
                stack,
            } => {
                let stack = stack.into_raw(cp)?;
                let frame_type = offset_delta + 64;
                if (64..=127).contains(&frame_type) {
                    let frame_type = u8::try_from(frame_type).expect("64 - 127 should fit in u8");
                    Self::Raw::SameLocals1StackItemFrame { frame_type, stack }
                } else {
                    Self::Raw::SameLocals1StackItemFrameExtended {
                        offset_delta,
                        stack,
                    }
                }
            }
            Self::ChopFrame {
                offset_delta,
                chop_count,
            } => Self::Raw::ChopFrame {
                offset_delta,
                frame_type: 251 - chop_count,
            },
            Self::AppendFrame {
                offset_delta,
                locals,
            } => Self::Raw::AppendFrame {
                offset_delta,
                locals: locals.into_iter().map(|it| it.into_raw(cp)).try_collect()?,
            },
            Self::FullFrame {
                offset_delta,
                locals,
                stack,
            } => Self::Raw::FullFrame {
                offset_delta,
                locals: locals.into_iter().map(|it| it.into_raw(cp)).try_collect()?,
                stack: stack.into_iter().map(|it| it.into_raw(cp)).try_collect()?,
            },
        };
        Ok(raw)
    }
}

impl ClassElement for VerificationType {
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
            Self::Raw::Object { class_info_index } => Ok(Self::ObjectVariable(
                ctx.constant_pool.get_class_ref(class_info_index)?,
            )),
            Self::Raw::Uninitialized { offset } => Ok(Self::UninitializedVariable {
                offset: ProgramCounter::from(offset),
            }),
        }
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, GenerationError> {
        match self {
            Self::TopVariable => Ok(Self::Raw::Top),
            Self::IntegerVariable => Ok(Self::Raw::Integer),
            Self::FloatVariable => Ok(Self::Raw::Float),
            Self::DoubleVariable => Ok(Self::Raw::Double),
            Self::LongVariable => Ok(Self::Raw::Long),
            Self::NullVariable => Ok(Self::Raw::Null),
            Self::UninitializedThisVariable => Ok(Self::Raw::UninitializedThis),
            Self::ObjectVariable(class) => Ok(Self::Raw::Object {
                class_info_index: cp.put_class_ref(class)?,
            }),
            Self::UninitializedVariable { offset } => Ok(Self::Raw::Uninitialized {
                offset: offset.into(),
            }),
        }
    }
}
