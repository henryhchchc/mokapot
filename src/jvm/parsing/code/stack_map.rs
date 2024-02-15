use std::iter::repeat_with;

use crate::jvm::{
    code::{StackMapFrame, VerificationTypeInfo},
    parsing::{
        jvm_element_parser::{parse_jvm, ParseJvmElement},
        parsing_context::ParsingContext,
        reader_utils::ClassReader,
        Error,
    },
};

impl<R: std::io::Read> ParseJvmElement<R> for StackMapFrame {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let frame_type: u8 = reader.read_value()?;
        let result = match frame_type {
            it @ 0..=63 => Self::SameFrame {
                offset_delta: u16::from(it),
            },
            it @ 64..=127 => Self::SameLocals1StackItemFrame {
                offset_delta: u16::from(it) - 64,
                stack: parse_jvm!(reader, ctx)?,
            },
            247 => {
                let offset_delta = reader.read_value()?;
                let stack = parse_jvm!(reader, ctx)?;
                Self::SameLocals1StackItemFrame {
                    offset_delta,
                    stack,
                }
            }
            it @ 248..=250 => {
                let chop_count = 251 - it;
                let offset_delta = reader.read_value()?;
                Self::ChopFrame {
                    chop_count,
                    offset_delta,
                }
            }
            251 => {
                let offset_delta = reader.read_value()?;
                Self::SameFrame { offset_delta }
            }
            it @ 252..=254 => {
                let offset_delta = reader.read_value()?;
                let locals_count = it - 251;
                let locals = repeat_with(|| parse_jvm!(reader, ctx))
                    .take(locals_count as usize)
                    .collect::<Result<_, _>>()?;
                Self::AppendFrame {
                    offset_delta,
                    locals,
                }
            }
            255 => {
                let offset_delta = reader.read_value()?;
                Self::FullFrame {
                    offset_delta,
                    locals: parse_jvm!(u16, reader, ctx)?,
                    stack: parse_jvm!(u16, reader, ctx)?,
                }
            }
            _ => Err(Error::UnknownStackMapFrameType(frame_type))?,
        };
        Ok(result)
    }
}

impl<R: std::io::Read> ParseJvmElement<R> for VerificationTypeInfo {
    fn parse(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let tag: u8 = reader.read_value()?;
        let result = match tag {
            0 => Self::TopVariable,
            1 => Self::IntegerVariable,
            2 => Self::FloatVariable,
            3 => Self::DoubleVariable,
            4 => Self::LongVariable,
            5 => Self::NullVariable,
            6 => Self::UninitializedThisVariable,
            7 => {
                let cpool_index = reader.read_value()?;
                let class_ref = ctx.constant_pool.get_class_ref(cpool_index)?;
                Self::ObjectVariable(class_ref)
            }
            8 => {
                let offset = reader.read_value::<u16>()?.into();
                Self::UninitializedVariable { offset }
            }
            unexpected => Err(Error::InvalidVerificationTypeInfoTag(unexpected))?,
        };
        Ok(result)
    }
}
