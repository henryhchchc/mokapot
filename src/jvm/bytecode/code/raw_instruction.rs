use std::{
    collections::{BTreeMap, VecDeque},
    io::{self, Read, Write},
    iter,
};

use super::super::{ParseError, reader_utils::BytecodeReader};
use crate::jvm::{
    bytecode::{errors::GenerationError, reader_utils::PositionTracker, write_length},
    code::{InstructionList, ProgramCounter, RawInstruction, RawWideInstruction},
};

impl InstructionList<RawInstruction> {
    /// Parses a list of [`RawInstruction`]s from the given bytes.
    /// # Errors
    /// See [`ParsingError`] for more information.
    pub fn from_bytes(bytes: Vec<u8>) -> Result<InstructionList<RawInstruction>, ParseError> {
        let bytes = VecDeque::from(bytes);
        let mut reader = PositionTracker::new(bytes);
        let inner: BTreeMap<_, _> =
            iter::from_fn(|| RawInstruction::read_one(&mut reader).transpose())
                .collect::<Result<_, _>>()?;
        Ok(InstructionList::from(inner))
    }

    /// Writes a list of [`RawInstruction`]s to the given writer.
    /// # Errors
    /// See [`ToWriterError`] for more information.
    pub fn to_writer<W: io::Write + ?Sized>(&self, writer: &mut W) -> Result<(), GenerationError> {
        let mut writer = PositionTracker::new(writer);

        for (_, insn) in self.iter() {
            insn.write_one(&mut writer)?;
        }

        Ok(())
    }
}

impl RawInstruction {
    /// Writes a single [`RawInstruction`] to the given writer.
    #[allow(clippy::too_many_lines)]
    fn write_one<W>(&self, writer: &mut PositionTracker<W>) -> Result<(), GenerationError>
    where
        PositionTracker<W>: io::Write,
    {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let opcode = self.opcode();
        writer.write_all(&opcode.to_be_bytes())?;

        match self {
            BiPush { value } => writer.write_all(&value.to_be_bytes())?,
            SiPush { value } => writer.write_all(&value.to_be_bytes())?,
            Ldc { const_index } => writer.write_all(&const_index.to_be_bytes())?,
            LdcW { const_index } | Ldc2W { const_index } => {
                writer.write_all(&const_index.to_be_bytes())?;
            }
            ILoad { index }
            | LLoad { index }
            | FLoad { index }
            | DLoad { index }
            | ALoad { index }
            | IStore { index }
            | LStore { index }
            | FStore { index }
            | DStore { index }
            | AStore { index } => writer.write_all(&index.to_be_bytes())?,
            IInc { index, constant } => {
                writer.write_all(&index.to_be_bytes())?;
                writer.write_all(&constant.to_be_bytes())?;
            }
            IfEq { offset }
            | IfNe { offset }
            | IfLt { offset }
            | IfGe { offset }
            | IfGt { offset }
            | IfLe { offset }
            | IfICmpEq { offset }
            | IfICmpNe { offset }
            | IfICmpLt { offset }
            | IfICmpGe { offset }
            | IfICmpGt { offset }
            | IfICmpLe { offset }
            | IfACmpEq { offset }
            | IfACmpNe { offset }
            | Goto { offset }
            | Jsr { offset } => writer.write_all(&offset.to_be_bytes())?,
            Ret { index } => writer.write_all(&index.to_be_bytes())?,
            TableSwitch {
                default,
                low,
                high,
                jump_offsets,
            } => {
                while writer.position() % 4 != 0 {
                    writer.write_all(&[0x00])?;
                }
                writer.write_all(&default.to_be_bytes())?;
                writer.write_all(&low.to_be_bytes())?;
                writer.write_all(&high.to_be_bytes())?;
                // No need to write the length, as it's implicitly determined by high - low + 1
                for offset in jump_offsets {
                    writer.write_all(&offset.to_be_bytes())?;
                }
            }
            LookupSwitch {
                default,
                match_offsets,
            } => {
                while writer.position() % 4 != 0 {
                    writer.write_all(&[0x00])?;
                }
                writer.write_all(&default.to_be_bytes())?;
                write_length::<i32>(writer, match_offsets.len())?;
                let mut sorted_match_offsets = match_offsets.clone();
                sorted_match_offsets.sort_by_key(|(key, _)| *key);
                for (key, offset) in sorted_match_offsets {
                    writer.write_all(&key.to_be_bytes())?;
                    writer.write_all(&offset.to_be_bytes())?;
                }
            }
            GetStatic { field_ref_index }
            | PutStatic { field_ref_index }
            | GetField { field_ref_index }
            | PutField { field_ref_index } => writer.write_all(&field_ref_index.to_be_bytes())?,
            InvokeVirtual { method_index }
            | InvokeSpecial { method_index }
            | InvokeStatic { method_index } => writer.write_all(&method_index.to_be_bytes())?,
            InvokeInterface {
                method_index,
                count,
            } => {
                writer.write_all(&method_index.to_be_bytes())?;
                writer.write_all(&count.to_be_bytes())?;
                writer.write_all(&[0x00])?;
            }
            InvokeDynamic { dynamic_index } => {
                writer.write_all(&dynamic_index.to_be_bytes())?;
                writer.write_all(&[0x00, 0x00])?;
            }
            New { index } => writer.write_all(&index.to_be_bytes())?,
            NewArray { atype } => writer.write_all(&atype.to_be_bytes())?,
            ANewArray { index } => writer.write_all(&index.to_be_bytes())?,
            CheckCast { target_type_index } => {
                writer.write_all(&target_type_index.to_be_bytes())?;
            }
            InstanceOf { target_type_index } => {
                writer.write_all(&target_type_index.to_be_bytes())?;
            }
            Wide(raw_wide_instruction) => {
                writer.write_all(&[raw_wide_instruction.opcode()])?;
                match raw_wide_instruction {
                    RawWideInstruction::ILoad { index }
                    | RawWideInstruction::LLoad { index }
                    | RawWideInstruction::FLoad { index }
                    | RawWideInstruction::DLoad { index }
                    | RawWideInstruction::ALoad { index }
                    | RawWideInstruction::IStore { index }
                    | RawWideInstruction::LStore { index }
                    | RawWideInstruction::FStore { index }
                    | RawWideInstruction::DStore { index }
                    | RawWideInstruction::AStore { index }
                    | RawWideInstruction::Ret { index } => {
                        writer.write_all(&index.to_be_bytes())?;
                    }
                    RawWideInstruction::IInc { index, increment } => {
                        writer.write_all(&index.to_be_bytes())?;
                        writer.write_all(&increment.to_be_bytes())?;
                    }
                }
            }
            MultiANewArray { index, dimensions } => {
                writer.write_all(&index.to_be_bytes())?;
                writer.write_all(&dimensions.to_be_bytes())?;
            }
            IfNull { offset } => writer.write_all(&offset.to_be_bytes())?,
            IfNonNull { offset } => writer.write_all(&offset.to_be_bytes())?,
            GotoW { offset } => writer.write_all(&offset.to_be_bytes())?,
            JsrW { offset } => writer.write_all(&offset.to_be_bytes())?,
            _ => {
                // Empty variants should write nothing but the opcode.
            }
        }
        Ok(())
    }

    /// Reads and parses a single [`RawInstruction`] from the given reader.
    #[allow(clippy::too_many_lines)]
    fn read_one<R>(
        reader: &mut PositionTracker<R>,
    ) -> Result<Option<(ProgramCounter, Self)>, ParseError>
    where
        PositionTracker<R>: Read,
    {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let pc = u16::try_from(reader.position())
            .map_err(|_| ParseError::malform("The instruction list is too long"))?
            .into();
        let opcode: u8 = match reader.decode_value() {
            Ok(it) => it,
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => Err(ParseError::from(e))?,
        };
        let instruction = match opcode {
            0x32 => AALoad,
            0x53 => AAStore,
            0x01 => AConstNull,
            0x19 => ALoad {
                index: reader.decode_value()?,
            },
            0x2a => ALoad0,
            0x2b => ALoad1,
            0x2c => ALoad2,
            0x2d => ALoad3,
            0xbd => ANewArray {
                index: reader.decode_value()?,
            },
            0xb0 => AReturn,
            0xbe => ArrayLength,
            0x3a => AStore {
                index: reader.decode_value()?,
            },
            0x4b => AStore0,
            0x4c => AStore1,
            0x4d => AStore2,
            0x4e => AStore3,
            0xbf => AThrow,
            0x33 => BALoad,
            0x54 => BAStore,
            0x10 => BiPush {
                value: reader.decode_value()?,
            },
            0x34 => CALoad,
            0x55 => CAStore,
            0xc0 => CheckCast {
                target_type_index: reader.decode_value()?,
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
                index: reader.decode_value()?,
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
                index: reader.decode_value()?,
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
                index: reader.decode_value()?,
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
                index: reader.decode_value()?,
            },
            0x43 => FStore0,
            0x44 => FStore1,
            0x45 => FStore2,
            0x46 => FStore3,
            0x66 => FSub,
            0xb4 => GetField {
                field_ref_index: reader.decode_value()?,
            },
            0xb2 => GetStatic {
                field_ref_index: reader.decode_value()?,
            },
            0xa7 => Goto {
                offset: reader.decode_value()?,
            },
            0xc8 => GotoW {
                offset: reader.decode_value()?,
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
                offset: reader.decode_value()?,
            },
            0xa6 => IfACmpNe {
                offset: reader.decode_value()?,
            },
            0x9f => IfICmpEq {
                offset: reader.decode_value()?,
            },
            0xa0 => IfICmpNe {
                offset: reader.decode_value()?,
            },
            0xa1 => IfICmpLt {
                offset: reader.decode_value()?,
            },
            0xa2 => IfICmpGe {
                offset: reader.decode_value()?,
            },
            0xa3 => IfICmpGt {
                offset: reader.decode_value()?,
            },
            0xa4 => IfICmpLe {
                offset: reader.decode_value()?,
            },
            0x99 => IfEq {
                offset: reader.decode_value()?,
            },
            0x9a => IfNe {
                offset: reader.decode_value()?,
            },
            0x9b => IfLt {
                offset: reader.decode_value()?,
            },
            0x9c => IfGe {
                offset: reader.decode_value()?,
            },
            0x9d => IfGt {
                offset: reader.decode_value()?,
            },
            0x9e => IfLe {
                offset: reader.decode_value()?,
            },
            0xc7 => IfNonNull {
                offset: reader.decode_value()?,
            },
            0xc6 => IfNull {
                offset: reader.decode_value()?,
            },
            0x84 => IInc {
                index: reader.decode_value()?,
                constant: reader.decode_value()?,
            },
            0x15 => ILoad {
                index: reader.decode_value()?,
            },
            0x1a => ILoad0,
            0x1b => ILoad1,
            0x1c => ILoad2,
            0x1d => ILoad3,
            0x68 => IMul,
            0x74 => INeg,
            0xc1 => InstanceOf {
                target_type_index: reader.decode_value()?,
            },
            0xba => {
                let dynamic_index = reader.decode_value()?;
                let zero: u16 = reader.decode_value()?;
                if zero != 0 {
                    ParseError::malform("Zero paddings are not zero");
                }
                InvokeDynamic { dynamic_index }
            }
            0xb9 => {
                let method_index = reader.decode_value()?;
                let count: u8 = reader.decode_value()?;
                let zero: u8 = reader.decode_value()?;
                if zero != 0 {
                    Err(ParseError::malform("Zero paddings are not zero"))?;
                }
                InvokeInterface {
                    method_index,
                    count,
                }
            }
            0xb7 => InvokeSpecial {
                method_index: reader.decode_value()?,
            },
            0xb8 => InvokeStatic {
                method_index: reader.decode_value()?,
            },
            0xb6 => InvokeVirtual {
                method_index: reader.decode_value()?,
            },
            0x80 => IOr,
            0x70 => IRem,
            0xac => IReturn,
            0x78 => IShl,
            0x7a => IShr,
            0x36 => IStore {
                index: reader.decode_value()?,
            },
            0x3b => IStore0,
            0x3c => IStore1,
            0x3d => IStore2,
            0x3e => IStore3,
            0x64 => ISub,
            0x7c => IUShr,
            0x82 => IXor,
            0xa8 => Jsr {
                offset: reader.decode_value()?,
            },
            0xc9 => JsrW {
                offset: reader.decode_value()?,
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
                const_index: reader.decode_value()?,
            },
            0x13 => LdcW {
                const_index: reader.decode_value()?,
            },
            0x14 => Ldc2W {
                const_index: reader.decode_value()?,
            },
            0x6d => LDiv,
            0x16 => LLoad {
                index: reader.decode_value()?,
            },
            0x1e => LLoad0,
            0x1f => LLoad1,
            0x20 => LLoad2,
            0x21 => LLoad3,
            0x69 => LMul,
            0x75 => LNeg,
            0xab => {
                while reader.position() % 4 != 0 {
                    let _padding_byte: u8 = reader.decode_value()?;
                }
                let default = reader.decode_value()?;
                let npairs = reader.decode_value()?;
                let match_offsets = (0..npairs)
                    .map(|_| {
                        let match_value = reader.decode_value()?;
                        let offset = reader.decode_value()?;
                        Ok((match_value, offset))
                    })
                    .collect::<io::Result<_>>()?;
                LookupSwitch {
                    default,
                    match_offsets,
                }
            }
            0xaa => {
                while reader.position() % 4 != 0 {
                    let _padding_byte: u8 = reader.decode_value()?;
                }
                let default = reader.decode_value()?;
                let low = reader.decode_value()?;
                let high = reader.decode_value()?;
                let jump_offsets = (low..=high)
                    .map(|_| reader.decode_value())
                    .collect::<io::Result<_>>()?;
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
                index: reader.decode_value()?,
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
                index: reader.decode_value()?,
                dimensions: reader.decode_value()?,
            },
            0xbb => New {
                index: reader.decode_value()?,
            },
            0xbc => NewArray {
                atype: reader.decode_value()?,
            },
            0x00 => Nop,
            0x57 => Pop,
            0x58 => Pop2,
            0xb5 => PutField {
                field_ref_index: reader.decode_value()?,
            },
            0xb3 => PutStatic {
                field_ref_index: reader.decode_value()?,
            },
            0xa9 => Ret {
                index: reader.decode_value()?,
            },
            0xb1 => Return,
            0x35 => SALoad,
            0x56 => SAStore,
            0x11 => SiPush {
                value: reader.decode_value()?,
            },
            0x5f => Swap,
            0xc4 => {
                let wide_opcode = reader.decode_value()?;
                let wide_insn = match wide_opcode {
                    0x15 => RawWideInstruction::ILoad {
                        index: reader.decode_value()?,
                    },
                    0x16 => RawWideInstruction::LLoad {
                        index: reader.decode_value()?,
                    },
                    0x17 => RawWideInstruction::FLoad {
                        index: reader.decode_value()?,
                    },
                    0x18 => RawWideInstruction::DLoad {
                        index: reader.decode_value()?,
                    },
                    0x19 => RawWideInstruction::ALoad {
                        index: reader.decode_value()?,
                    },
                    0x36 => RawWideInstruction::IStore {
                        index: reader.decode_value()?,
                    },
                    0x37 => RawWideInstruction::LStore {
                        index: reader.decode_value()?,
                    },
                    0x38 => RawWideInstruction::FStore {
                        index: reader.decode_value()?,
                    },
                    0x39 => RawWideInstruction::DStore {
                        index: reader.decode_value()?,
                    },
                    0x3a => RawWideInstruction::AStore {
                        index: reader.decode_value()?,
                    },
                    0x84 => RawWideInstruction::IInc {
                        index: reader.decode_value()?,
                        increment: reader.decode_value()?,
                    },
                    0xa9 => RawWideInstruction::Ret {
                        index: reader.decode_value()?,
                    },
                    _ => Err(ParseError::malform("Invalid opcode"))?,
                };
                Wide(wide_insn)
            }
            _ => Err(ParseError::malform("Invalid opcode"))?,
        };
        Ok(Some((pc, instruction)))
    }

    /// Returns the number of bytes occupied by the instruction given its location in the instruction list.
    pub(crate) fn num_bytes(&self, pc: ProgramCounter) -> Result<u16, GenerationError> {
        #[allow(clippy::enum_glob_use, reason = "It's long by definition")]
        use RawInstruction::*;

        Ok(match self {
            // Instructions that are just the opcode (1 byte)
            AALoad | AAStore | AConstNull | ALoad0 | ALoad1 | ALoad2 | ALoad3 | AReturn
            | ArrayLength | AStore0 | AStore1 | AStore2 | AStore3 | AThrow | BALoad | BAStore
            | CALoad | CAStore | D2F | D2I | D2L | DAdd | DALoad | DAStore | DCmpG | DCmpL
            | DConst0 | DConst1 | DDiv | DLoad0 | DLoad1 | DLoad2 | DLoad3 | DMul | DNeg | DRem
            | DReturn | DStore0 | DStore1 | DStore2 | DStore3 | DSub | Dup | DupX1 | DupX2
            | Dup2 | Dup2X1 | Dup2X2 | F2D | F2I | F2L | FAdd | FALoad | FAStore | FCmpG
            | FCmpL | FConst0 | FConst1 | FConst2 | FDiv | FLoad0 | FLoad1 | FLoad2 | FLoad3
            | FMul | FNeg | FRem | FReturn | FStore0 | FStore1 | FStore2 | FStore3 | FSub | I2B
            | I2C | I2D | I2F | I2L | I2S | IAdd | IALoad | IAnd | IAStore | IConstM1 | IConst0
            | IConst1 | IConst2 | IConst3 | IConst4 | IConst5 | IDiv | ILoad0 | ILoad1 | ILoad2
            | ILoad3 | IMul | INeg | IOr | IRem | IReturn | IShl | IShr | IStore0 | IStore1
            | IStore2 | IStore3 | ISub | IUShr | IXor | L2D | L2F | L2I | LAdd | LALoad | LAnd
            | LAStore | LCmp | LConst0 | LConst1 | LDiv | LLoad0 | LLoad1 | LLoad2 | LLoad3
            | LMul | LNeg | LOr | LRem | LReturn | LShl | LShr | LStore0 | LStore1 | LStore2
            | LStore3 | LSub | LUShr | LXor | MonitorEnter | MonitorExit | Nop | Pop | Pop2
            | Return | SALoad | SAStore | Swap | Breakpoint | ImpDep1 | ImpDep2 => 1,

            // Instructions that are opcode + 1 byte
            BiPush { .. }
            | Ldc { .. }
            | ALoad { .. }
            | FLoad { .. }
            | DLoad { .. }
            | ILoad { .. }
            | LLoad { .. }
            | AStore { .. }
            | FStore { .. }
            | DStore { .. }
            | IStore { .. }
            | LStore { .. }
            | NewArray { .. }
            | Ret { .. } => 2,

            // Instructions that are opcode + 2 bytes
            SiPush { .. }
            | LdcW { .. }
            | Ldc2W { .. }
            | GetField { .. }
            | GetStatic { .. }
            | PutField { .. }
            | PutStatic { .. }
            | InvokeVirtual { .. }
            | InvokeSpecial { .. }
            | InvokeStatic { .. }
            | New { .. }
            | ANewArray { .. }
            | CheckCast { .. }
            | InstanceOf { .. }
            | Goto { .. }
            | IfEq { .. }
            | IfNe { .. }
            | IfLt { .. }
            | IfGe { .. }
            | IfGt { .. }
            | IfLe { .. }
            | IfICmpEq { .. }
            | IfICmpNe { .. }
            | IfICmpLt { .. }
            | IfICmpGe { .. }
            | IfICmpGt { .. }
            | IfICmpLe { .. }
            | IfACmpEq { .. }
            | IfACmpNe { .. }
            | IfNull { .. }
            | IfNonNull { .. }
            | Jsr { .. }
            | IInc { .. }
            | MultiANewArray { .. } => 3,

            InvokeInterface { .. } | InvokeDynamic { .. } | GotoW { .. } | JsrW { .. } => 5,

            // For WIDE instruction, depend on sub-instruction
            Wide(wide_insn) => {
                // 1 byte for wide opcode + 1 byte for sub-opcode + size of operands
                match wide_insn {
                    RawWideInstruction::IInc { .. } => 6, // wide + iinc + 2-byte index + 2-byte const
                    _ => 4,                               // wide + sub-opcode + 2-byte index
                }
            }

            // Variable-length instructions require special handling
            TableSwitch { low, high, .. } => {
                let padding = (4 - (u16::from(pc) + 1) % 4) % 4; // padding after opcode
                let entries = u16::try_from(*high - *low + 1)
                    .map_err(|_| GenerationError::other("Invalid jump offset"))?;
                1 + padding + 12 + (4 * entries) // opcode + padding + default,low,high + entries
            }

            LookupSwitch { match_offsets, .. } => {
                let padding = (4 - (u16::from(pc) + 1) % 4) % 4; // padding after opcode
                let entries = u16::try_from(match_offsets.len())
                    .map_err(|_| GenerationError::other("Invalid jump offset"))?;
                1 + padding + 8 + (8 * entries) // opcode + padding + default,npairs + (match,offset) pairs
            }
        })
    }
}
