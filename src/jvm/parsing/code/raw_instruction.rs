use std::{collections::BTreeMap, io::Cursor};

use super::super::{reader_utils::ValueReaderExt, Error};
use crate::{
    jvm::code::{InstructionList, ProgramCounter, RawInstruction, RawWideInstruction},
    macros::malform,
};

impl RawInstruction {
    /// Parses a list of [`RawInstruction`]s from the given bytes.
    /// # Errors
    /// See [`Error`] for more information.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<InstructionList<RawInstruction>, Error> {
        let mut cursor = Cursor::new(bytes);
        let mut inner = BTreeMap::new();
        while let Some((pc, instruction)) = RawInstruction::parse(&mut cursor)? {
            inner.insert(pc, instruction);
        }
        Ok(InstructionList::from(inner))
    }

    #[allow(clippy::too_many_lines)]
    fn parse(reader: &mut Cursor<Vec<u8>>) -> Result<Option<(ProgramCounter, Self)>, Error> {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let pc = u16::try_from(reader.position())
            .map_err(|_| Error::TooLongInstructionList)?
            .into();
        let opcode: u8 = match reader.read_value() {
            Ok(it) => it,
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => Err(Error::ReadFail(e))?,
        };
        let instruction = match opcode {
            0x32 => AALoad,
            0x53 => AAStore,
            0x01 => AConstNull,
            0x19 => ALoad {
                index: reader.read_value()?,
            },
            0x2a => ALoad0,
            0x2b => ALoad1,
            0x2c => ALoad2,
            0x2d => ALoad3,
            0xbd => ANewArray {
                index: reader.read_value()?,
            },
            0xb0 => AReturn,
            0xbe => ArrayLength,
            0x3a => AStore {
                index: reader.read_value()?,
            },
            0x4b => AStore0,
            0x4c => AStore1,
            0x4d => AStore2,
            0x4e => AStore3,
            0xbf => AThrow,
            0x33 => BALoad,
            0x54 => BAStore,
            0x10 => BiPush {
                value: reader.read_value()?,
            },
            0x34 => CALoad,
            0x55 => CAStore,
            0xc0 => CheckCast {
                target_type_index: reader.read_value()?,
            },
            0x90 => D2F,
            0x8e => D2I,
            0x8f => D2L,
            0x63 => DAdd,
            0x31 => DALoad,
            0x52 => DAStore,
            0x98 => DCmpG,
            0x97 => DCmpL,
            0x0e => DConst0,
            0x0f => DConst1,
            0x6f => DDiv,
            0x18 => DLoad {
                index: reader.read_value()?,
            },
            0x26 => DLoad0,
            0x27 => DLoad1,
            0x28 => DLoad2,
            0x29 => DLoad3,
            0x6b => DMul,
            0x77 => DNeg,
            0x73 => DRem,
            0xaf => DReturn,
            0x39 => DStore {
                index: reader.read_value()?,
            },
            0x47 => DStore0,
            0x48 => DStore1,
            0x49 => DStore2,
            0x4a => DStore3,
            0x67 => DSub,
            0x59 => Dup,
            0x5a => DupX1,
            0x5b => DupX2,
            0x5c => Dup2,
            0x5d => Dup2X1,
            0x5e => Dup2X2,
            0x8d => F2D,
            0x8b => F2I,
            0x8c => F2L,
            0x62 => FAdd,
            0x30 => FALoad,
            0x51 => FAStore,
            0x96 => FCmpG,
            0x95 => FCmpL,
            0x0b => FConst0,
            0x0c => FConst1,
            0x0d => FConst2,
            0x6e => FDiv,
            0x17 => FLoad {
                index: reader.read_value()?,
            },
            0x22 => FLoad0,
            0x23 => FLoad1,
            0x24 => FLoad2,
            0x25 => FLoad3,
            0x6a => FMul,
            0x76 => FNeg,
            0x72 => FRem,
            0xae => FReturn,
            0x38 => FStore {
                index: reader.read_value()?,
            },
            0x43 => FStore0,
            0x44 => FStore1,
            0x45 => FStore2,
            0x46 => FStore3,
            0x66 => FSub,
            0xb4 => GetField {
                field_ref_index: reader.read_value()?,
            },
            0xb2 => GetStatic {
                field_ref_index: reader.read_value()?,
            },
            0xa7 => Goto {
                offset: reader.read_value()?,
            },
            0xc8 => GotoW {
                offset: reader.read_value()?,
            },
            0x91 => I2B,
            0x92 => I2C,
            0x87 => I2D,
            0x86 => I2F,
            0x85 => I2L,
            0x93 => I2S,
            0x60 => IAdd,
            0x2e => IALoad,
            0x7e => IAnd,
            0x4f => IAStore,
            0x02 => IConstM1,
            0x03 => IConst0,
            0x04 => IConst1,
            0x05 => IConst2,
            0x06 => IConst3,
            0x07 => IConst4,
            0x08 => IConst5,
            0x6c => IDiv,
            0xa5 => IfACmpEq {
                offset: reader.read_value()?,
            },
            0xa6 => IfACmpNe {
                offset: reader.read_value()?,
            },
            0x9f => IfICmpEq {
                offset: reader.read_value()?,
            },
            0xa0 => IfICmpNe {
                offset: reader.read_value()?,
            },
            0xa1 => IfICmpLt {
                offset: reader.read_value()?,
            },
            0xa2 => IfICmpGe {
                offset: reader.read_value()?,
            },
            0xa3 => IfICmpGt {
                offset: reader.read_value()?,
            },
            0xa4 => IfICmpLe {
                offset: reader.read_value()?,
            },
            0x99 => IfEq {
                offset: reader.read_value()?,
            },
            0x9a => IfNe {
                offset: reader.read_value()?,
            },
            0x9b => IfLt {
                offset: reader.read_value()?,
            },
            0x9c => IfGe {
                offset: reader.read_value()?,
            },
            0x9d => IfGt {
                offset: reader.read_value()?,
            },
            0x9e => IfLe {
                offset: reader.read_value()?,
            },
            0xc7 => IfNonNull {
                offset: reader.read_value()?,
            },
            0xc6 => IfNull {
                offset: reader.read_value()?,
            },
            0x84 => IInc {
                index: reader.read_value()?,
                constant: reader.read_value()?,
            },
            0x15 => ILoad {
                index: reader.read_value()?,
            },
            0x1a => ILoad0,
            0x1b => ILoad1,
            0x1c => ILoad2,
            0x1d => ILoad3,
            0x68 => IMul,
            0x74 => INeg,
            0xc1 => InstanceOf {
                target_type_index: reader.read_value()?,
            },
            0xba => {
                let dynamic_index = reader.read_value()?;
                let zero: u16 = reader.read_value()?;
                if zero != 0 {
                    malform!("Zero paddings are not zero");
                }
                InvokeDynamic { dynamic_index }
            }
            0xb9 => {
                let method_index = reader.read_value()?;
                let count: u8 = reader.read_value()?;
                let zero: u8 = reader.read_value()?;
                if zero != 0 {
                    malform!("Zero paddings are not zero");
                }
                InvokeInterface {
                    method_index,
                    count,
                }
            }
            0xb7 => InvokeSpecial {
                method_index: reader.read_value()?,
            },
            0xb8 => InvokeStatic {
                method_index: reader.read_value()?,
            },
            0xb6 => InvokeVirtual {
                method_index: reader.read_value()?,
            },
            0x80 => IOr,
            0x70 => IRem,
            0xac => IReturn,
            0x78 => IShl,
            0x7a => IShr,
            0x36 => IStore {
                index: reader.read_value()?,
            },
            0x3b => IStore0,
            0x3c => IStore1,
            0x3d => IStore2,
            0x3e => IStore3,
            0x64 => ISub,
            0x7c => IUShr,
            0x82 => IXor,
            0xa8 => Jsr {
                offset: reader.read_value()?,
            },
            0xc9 => JsrW {
                offset: reader.read_value()?,
            },
            0x8a => L2D,
            0x89 => L2F,
            0x88 => L2I,
            0x61 => LAdd,
            0x2f => LALoad,
            0x7f => LAnd,
            0x50 => LAStore,
            0x94 => LCmp,
            0x09 => LConst0,
            0x0a => LConst1,
            0x12 => Ldc {
                const_index: reader.read_value()?,
            },
            0x13 => LdcW {
                const_index: reader.read_value()?,
            },
            0x14 => Ldc2W {
                const_index: reader.read_value()?,
            },
            0x6d => LDiv,
            0x16 => LLoad {
                index: reader.read_value()?,
            },
            0x1e => LLoad0,
            0x1f => LLoad1,
            0x20 => LLoad2,
            0x21 => LLoad3,
            0x69 => LMul,
            0x75 => LNeg,
            0xab => {
                while reader.position() % 4 != 0 {
                    let _padding_byte: u8 = reader.read_value()?;
                }
                let default = reader.read_value()?;
                let npairs = reader.read_value()?;
                let match_offsets = (0..npairs)
                    .map(|_| {
                        let match_value = reader.read_value()?;
                        let offset = reader.read_value()?;
                        Ok((match_value, offset))
                    })
                    .collect::<Result<_, Error>>()?;
                LookupSwitch {
                    default,
                    match_offsets,
                }
            }
            0xaa => {
                while reader.position() % 4 != 0 {
                    let _padding_byte: u8 = reader.read_value()?;
                }
                let default = reader.read_value()?;
                let low = reader.read_value()?;
                let high = reader.read_value()?;
                let jump_offsets = (low..=high)
                    .map(|_| reader.read_value())
                    .collect::<std::io::Result<_>>()?;
                TableSwitch {
                    default,
                    low,
                    high,
                    jump_offsets,
                }
            }
            0x81 => LOr,
            0x71 => LRem,
            0xad => LReturn,
            0x79 => LShl,
            0x7b => LShr,
            0x37 => LStore {
                index: reader.read_value()?,
            },
            0x3f => LStore0,
            0x40 => LStore1,
            0x41 => LStore2,
            0x42 => LStore3,
            0x65 => LSub,
            0x7d => LUShr,
            0x83 => LXor,
            0xc2 => MonitorEnter,
            0xc3 => MonitorExit,
            0xc5 => MultiANewArray {
                index: reader.read_value()?,
                dimensions: reader.read_value()?,
            },
            0xbb => New {
                index: reader.read_value()?,
            },
            0xbc => NewArray {
                atype: reader.read_value()?,
            },
            0x00 => Nop,
            0x57 => Pop,
            0x58 => Pop2,
            0xb5 => PutField {
                field_ref_index: reader.read_value()?,
            },
            0xb3 => PutStatic {
                field_ref_index: reader.read_value()?,
            },
            0xa9 => Ret {
                index: reader.read_value()?,
            },
            0xb1 => Return,
            0x35 => SALoad,
            0x56 => SAStore,
            0x11 => SiPush {
                value: reader.read_value()?,
            },
            0x5f => Swap,
            0xc4 => {
                let wide_opcode = reader.read_value()?;
                let wide_insn = match wide_opcode {
                    0x15 => RawWideInstruction::ILoad {
                        index: reader.read_value()?,
                    },
                    0x16 => RawWideInstruction::LLoad {
                        index: reader.read_value()?,
                    },
                    0x17 => RawWideInstruction::FLoad {
                        index: reader.read_value()?,
                    },
                    0x18 => RawWideInstruction::DLoad {
                        index: reader.read_value()?,
                    },
                    0x19 => RawWideInstruction::ALoad {
                        index: reader.read_value()?,
                    },
                    0x36 => RawWideInstruction::IStore {
                        index: reader.read_value()?,
                    },
                    0x37 => RawWideInstruction::LStore {
                        index: reader.read_value()?,
                    },
                    0x38 => RawWideInstruction::FStore {
                        index: reader.read_value()?,
                    },
                    0x39 => RawWideInstruction::DStore {
                        index: reader.read_value()?,
                    },
                    0x3a => RawWideInstruction::AStore {
                        index: reader.read_value()?,
                    },
                    0x84 => RawWideInstruction::IInc {
                        index: reader.read_value()?,
                        increment: reader.read_value()?,
                    },
                    0xa9 => RawWideInstruction::Ret {
                        index: reader.read_value()?,
                    },
                    _ => Err(Error::UnexpectedOpCode(wide_opcode))?,
                };
                Wide(wide_insn)
            }
            it => Err(Error::UnexpectedOpCode(it))?,
        };
        Ok(Some((pc, instruction)))
    }
}
