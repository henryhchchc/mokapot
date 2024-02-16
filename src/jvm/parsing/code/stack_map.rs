use std::{io::Read, iter::repeat_with};

use crate::jvm::{
    code::{StackMapFrame, VerificationTypeInfo},
    parsing::{
        jvm_element_parser::JvmElement, parsing_context::ParsingContext,
        reader_utils::ValueReaderExt, Error,
    },
};

impl JvmElement for StackMapFrame {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
        let frame_type: u8 = reader.read_value()?;
        let result = match frame_type {
            it @ 0..=63 => Self::SameFrame {
                offset_delta: u16::from(it),
            },
            it @ 64..=127 => Self::SameLocals1StackItemFrame {
                offset_delta: u16::from(it) - 64,
                stack: JvmElement::parse(reader, ctx)?,
            },
            247 => {
                let offset_delta = reader.read_value()?;
                let stack = JvmElement::parse(reader, ctx)?;
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
                let locals = repeat_with(|| JvmElement::parse(reader, ctx))
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
                    locals: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
                    stack: JvmElement::parse_vec::<u16, _>(reader, ctx)?,
                }
            }
            _ => Err(Error::UnknownStackMapFrameType(frame_type))?,
        };
        Ok(result)
    }
}

impl JvmElement for VerificationTypeInfo {
    fn parse<R: Read + ?Sized>(reader: &mut R, ctx: &ParsingContext) -> Result<Self, Error> {
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
                let offset = reader.read_value()?;
                Self::UninitializedVariable { offset }
            }
            unexpected => Err(Error::InvalidVerificationTypeInfoTag(unexpected))?,
        };
        Ok(result)
    }
}
