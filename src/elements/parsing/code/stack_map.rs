use crate::{
    elements::{
        instruction::{StackMapFrame, VerificationTypeInfo},
        parsing::constant_pool::ParsingContext,
    },
    errors::ClassFileParsingError,
    reader_utils::{read_u16, read_u8},
};

impl StackMapFrame {
    pub(crate) fn parse<R>(
        reader: &mut R,
        ctx: &ParsingContext,
    ) -> Result<StackMapFrame, ClassFileParsingError>
    where
        R: std::io::Read,
    {
        let frame_type = read_u8(reader)?;
        let result = match frame_type {
            0..=63 => Self::SameFrame {
                offset_delta: frame_type as u16,
            },
            64..=127 => Self::SameLocals1StackItemFrame(VerificationTypeInfo::parse(reader, ctx)?),
            247 => {
                let offset_delta = read_u16(reader)?;
                let stack = VerificationTypeInfo::parse(reader, ctx)?;
                Self::Semantics1StackItemFrameExtended(offset_delta, stack)
            }
            248..=250 => {
                let chop_count = 251 - frame_type;
                let offset_delta = read_u16(reader)?;
                Self::ChopFrame {
                    chop_count,
                    offset_delta,
                }
            }
            251 => {
                let offset_delta = read_u16(reader)?;
                Self::SameFrameExtended { offset_delta }
            }
            252..=254 => {
                let offset_delta = read_u16(reader)?;
                let locals_count = frame_type - 251;
                let mut locals = Vec::with_capacity(locals_count as usize);
                for _ in 0..locals_count {
                    let local = VerificationTypeInfo::parse(reader, ctx)?;
                    locals.push(local);
                }
                Self::AppendFrame {
                    offset_delta,
                    locals,
                }
            }
            255 => {
                let offset_delta = read_u16(reader)?;
                let locals_count = read_u16(reader)?;
                let mut locals = Vec::with_capacity(locals_count as usize);
                for _ in 0..locals_count {
                    let local = VerificationTypeInfo::parse(reader, ctx)?;
                    locals.push(local);
                }
                let stacks_count = read_u16(reader)?;
                let mut stack = Vec::with_capacity(stacks_count as usize);
                for _ in 0..stacks_count {
                    let stack_element = VerificationTypeInfo::parse(reader, ctx)?;
                    stack.push(stack_element)
                }
                Self::FullFrame {
                    offset_delta,
                    locals,
                    stack,
                }
            }
            _ => Err(ClassFileParsingError::UnknownStackMapFrameType(frame_type))?,
        };
        Ok(result)
    }
}
