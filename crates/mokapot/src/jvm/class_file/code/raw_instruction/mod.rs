//! Raw JVM instruction model and parser.

use crate::jvm::class_file::{Decode, decoder::decode_value};
use zerocopy::{FromBytes, Immutable, KnownLayout, Unaligned, byteorder::big_endian::I32};

use std::iter;

use super::super::{ParseError, fixed_layout::FixedLayout};
use crate::{intrinsics::see_jvm_spec, jvm::code::ProgramCounter};

/// A JVM instruction before resolving constant-pool references.
/// Switch tables borrow their wire data from the input bytecode.
#[doc = see_jvm_spec!(6, 5)]
#[repr(u8)]
#[allow(
    missing_docs,
    reason = "This maps one-to-one to those in the JVM spec."
)]
#[derive(Debug, PartialEq, Clone, Copy)]
pub enum RawInstruction<'a> {
    Nop = 0x00,
    AConstNull = 0x01,
    IConstM1 = 0x02,
    IConst0 = 0x03,
    IConst1 = 0x04,
    IConst2 = 0x05,
    IConst3 = 0x06,
    IConst4 = 0x07,
    IConst5 = 0x08,
    LConst0 = 0x09,
    LConst1 = 0x0A,
    FConst0 = 0x0B,
    FConst1 = 0x0C,
    FConst2 = 0x0D,
    DConst0 = 0x0E,
    DConst1 = 0x0F,
    BiPush { value: i8 } = 0x10,
    SiPush { value: i16 } = 0x11,
    Ldc { const_index: u8 } = 0x12,
    LdcW { const_index: u16 } = 0x13,
    Ldc2W { const_index: u16 } = 0x14,
    ILoad { index: u8 } = 0x15,
    LLoad { index: u8 } = 0x16,
    FLoad { index: u8 } = 0x17,
    DLoad { index: u8 } = 0x18,
    ALoad { index: u8 } = 0x19,
    ILoad0 = 0x1A,
    ILoad1 = 0x1B,
    ILoad2 = 0x1C,
    ILoad3 = 0x1D,
    LLoad0 = 0x1E,
    LLoad1 = 0x1F,
    LLoad2 = 0x20,
    LLoad3 = 0x21,
    FLoad0 = 0x22,
    FLoad1 = 0x23,
    FLoad2 = 0x24,
    FLoad3 = 0x25,
    DLoad0 = 0x26,
    DLoad1 = 0x27,
    DLoad2 = 0x28,
    DLoad3 = 0x29,
    ALoad0 = 0x2A,
    ALoad1 = 0x2B,
    ALoad2 = 0x2C,
    ALoad3 = 0x2D,
    IALoad = 0x2E,
    LALoad = 0x2F,
    FALoad = 0x30,
    DALoad = 0x31,
    AALoad = 0x32,
    BALoad = 0x33,
    CALoad = 0x34,
    SALoad = 0x35,
    IStore { index: u8 } = 0x36,
    LStore { index: u8 } = 0x37,
    FStore { index: u8 } = 0x38,
    DStore { index: u8 } = 0x39,
    AStore { index: u8 } = 0x3A,
    IStore0 = 0x3B,
    IStore1 = 0x3C,
    IStore2 = 0x3D,
    IStore3 = 0x3E,
    LStore0 = 0x3F,
    LStore1 = 0x40,
    LStore2 = 0x41,
    LStore3 = 0x42,
    FStore0 = 0x43,
    FStore1 = 0x44,
    FStore2 = 0x45,
    FStore3 = 0x46,
    DStore0 = 0x47,
    DStore1 = 0x48,
    DStore2 = 0x49,
    DStore3 = 0x4A,
    AStore0 = 0x4B,
    AStore1 = 0x4C,
    AStore2 = 0x4D,
    AStore3 = 0x4E,
    IAStore = 0x4F,
    LAStore = 0x50,
    FAStore = 0x51,
    DAStore = 0x52,
    AAStore = 0x53,
    BAStore = 0x54,
    CAStore = 0x55,
    SAStore = 0x56,
    Pop = 0x57,
    Pop2 = 0x58,
    Dup = 0x59,
    DupX1 = 0x5A,
    DupX2 = 0x5B,
    Dup2 = 0x5C,
    Dup2X1 = 0x5D,
    Dup2X2 = 0x5E,
    Swap = 0x5F,
    IAdd = 0x60,
    LAdd = 0x61,
    FAdd = 0x62,
    DAdd = 0x63,
    ISub = 0x64,
    LSub = 0x65,
    FSub = 0x66,
    DSub = 0x67,
    IMul = 0x68,
    LMul = 0x69,
    FMul = 0x6A,
    DMul = 0x6B,
    IDiv = 0x6C,
    LDiv = 0x6D,
    FDiv = 0x6E,
    DDiv = 0x6F,
    IRem = 0x70,
    LRem = 0x71,
    FRem = 0x72,
    DRem = 0x73,
    INeg = 0x74,
    LNeg = 0x75,
    FNeg = 0x76,
    DNeg = 0x77,
    IShl = 0x78,
    LShl = 0x79,
    IShr = 0x7A,
    LShr = 0x7B,
    IUShr = 0x7C,
    LUShr = 0x7D,
    IAnd = 0x7E,
    LAnd = 0x7F,
    IOr = 0x80,
    LOr = 0x81,
    IXor = 0x82,
    LXor = 0x83,
    IInc { index: u8, constant: i8 } = 0x84,
    I2L = 0x85,
    I2F = 0x86,
    I2D = 0x87,
    L2I = 0x88,
    L2F = 0x89,
    L2D = 0x8A,
    F2I = 0x8B,
    F2L = 0x8C,
    F2D = 0x8D,
    D2I = 0x8E,
    D2L = 0x8F,
    D2F = 0x90,
    I2B = 0x91,
    I2C = 0x92,
    I2S = 0x93,
    LCmp = 0x94,
    FCmpL = 0x95,
    FCmpG = 0x96,
    DCmpL = 0x97,
    DCmpG = 0x98,
    IfEq { offset: i16 } = 0x99,
    IfNe { offset: i16 } = 0x9A,
    IfLt { offset: i16 } = 0x9B,
    IfGe { offset: i16 } = 0x9C,
    IfGt { offset: i16 } = 0x9D,
    IfLe { offset: i16 } = 0x9E,
    IfICmpEq { offset: i16 } = 0x9F,
    IfICmpNe { offset: i16 } = 0xA0,
    IfICmpLt { offset: i16 } = 0xA1,
    IfICmpGe { offset: i16 } = 0xA2,
    IfICmpGt { offset: i16 } = 0xA3,
    IfICmpLe { offset: i16 } = 0xA4,
    IfACmpEq { offset: i16 } = 0xA5,
    IfACmpNe { offset: i16 } = 0xA6,
    Goto { offset: i16 } = 0xA7,
    Jsr { offset: i16 } = 0xA8,
    Ret { index: u8 } = 0xA9,
    TableSwitch(&'a TableSwitchInfo) = 0xAA,
    LookupSwitch(&'a LookupSwitchInfo) = 0xAB,
    IReturn = 0xAC,
    LReturn = 0xAD,
    FReturn = 0xAE,
    DReturn = 0xAF,
    AReturn = 0xB0,
    Return = 0xB1,
    GetStatic { field_ref_index: u16 } = 0xB2,
    PutStatic { field_ref_index: u16 } = 0xB3,
    GetField { field_ref_index: u16 } = 0xB4,
    PutField { field_ref_index: u16 } = 0xB5,
    InvokeVirtual { method_index: u16 } = 0xB6,
    InvokeSpecial { method_index: u16 } = 0xB7,
    InvokeStatic { method_index: u16 } = 0xB8,
    InvokeInterface { method_index: u16, count: u8 } = 0xB9,
    InvokeDynamic { dynamic_index: u16 } = 0xBA,
    New { index: u16 } = 0xBB,
    NewArray { atype: u8 } = 0xBC,
    ANewArray { index: u16 } = 0xBD,
    ArrayLength = 0xBE,
    AThrow = 0xBF,
    CheckCast { target_type_index: u16 } = 0xC0,
    InstanceOf { target_type_index: u16 } = 0xC1,
    MonitorEnter = 0xC2,
    MonitorExit = 0xC3,
    Wide(RawWideInstruction) = 0xC4,
    MultiANewArray { index: u16, dimensions: u8 } = 0xC5,
    IfNull { offset: i16 } = 0xC6,
    IfNonNull { offset: i16 } = 0xC7,
    GotoW { offset: i32 } = 0xC8,
    JsrW { offset: i32 } = 0xC9,
    Breakpoint = 0xCA,
    ImpDep1 = 0xFE,
    ImpDep2 = 0xFF,
}

/// A wide instruction.
#[doc = see_jvm_spec!(6, 5)]
#[allow(
    missing_docs,
    reason = "This maps one-to-one to those in the JVM spec."
)]
#[allow(
    clippy::module_name_repetitions,
    reason = "For consistent type names with the JVM spec."
)]
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
#[repr(u8)]
pub enum RawWideInstruction {
    ILoad { index: u16 } = 0x15,
    LLoad { index: u16 } = 0x16,
    FLoad { index: u16 } = 0x17,
    DLoad { index: u16 } = 0x18,
    ALoad { index: u16 } = 0x19,
    IStore { index: u16 } = 0x36,
    LStore { index: u16 } = 0x37,
    FStore { index: u16 } = 0x38,
    DStore { index: u16 } = 0x39,
    AStore { index: u16 } = 0x3A,
    Ret { index: u16 } = 0xA9,
    IInc { index: u16, increment: i16 } = 0x84,
}

/// A match value and relative jump offset in a lookup-switch wire table.
#[derive(Debug, Clone, Copy, PartialEq, Eq, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct LookupSwitchPair {
    /// The signed match value, in JVM byte order.
    pub match_value: I32,
    /// The signed jump offset relative to the switch instruction.
    pub offset: I32,
}

impl FixedLayout for LookupSwitchPair {}

/// A table-switch payload after its opcode and padding.
#[derive(Debug, PartialEq, Eq, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct TableSwitchInfo {
    /// The default relative jump offset.
    pub default: I32,
    /// The lowest match value.
    pub low: I32,
    /// The highest match value.
    pub high: I32,
    /// Relative jump offsets for the inclusive match range.
    pub jump_offsets: [I32],
}

impl<'a> Decode<'a> for &'a TableSwitchInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: i32 = decode_value(&mut header)?;
        let low: i32 = decode_value(&mut header)?;
        let high: i32 = decode_value(&mut header)?;
        if low > high {
            return Err(ParseError::malform("Invalid tableswitch range"));
        }
        let count = usize::try_from(i64::from(high) - i64::from(low) + 1)
            .map_err(|_| ParseError::malform("Invalid tableswitch range"))?;
        let (info, remaining) = TableSwitchInfo::ref_from_prefix_with_elems(bytes, count)
            .map_err(|_| ParseError::malform("Truncated tableswitch payload"))?;
        *bytes = remaining;
        Ok(info)
    }
}

/// A lookup-switch payload after its opcode and padding.
#[derive(Debug, PartialEq, Eq, FromBytes, Immutable, KnownLayout, Unaligned)]
#[repr(C)]
pub struct LookupSwitchInfo {
    /// The default relative jump offset.
    pub default: I32,
    /// The number of match-offset pairs.
    pub npairs: I32,
    /// Match values and their relative jump offsets, ordered by match value.
    pub match_offsets: [LookupSwitchPair],
}

impl<'a> Decode<'a> for &'a LookupSwitchInfo {
    fn decode(bytes: &mut &'a [u8]) -> Result<Self, ParseError> {
        let mut header = *bytes;
        let _: i32 = decode_value(&mut header)?;
        let npairs: i32 = decode_value(&mut header)?;
        let count = usize::try_from(npairs)
            .map_err(|_| ParseError::malform("Invalid lookupswitch pair count"))?;
        let (info, remaining) = LookupSwitchInfo::ref_from_prefix_with_elems(bytes, count)
            .map_err(|_| ParseError::malform("Truncated lookupswitch payload"))?;
        if info
            .match_offsets
            .windows(2)
            .any(|pair| pair[0].match_value.get() >= pair[1].match_value.get())
        {
            return Err(ParseError::malform(
                "Lookupswitch match values are not strictly increasing",
            ));
        }
        *bytes = remaining;
        Ok(info)
    }
}

impl<'a> RawInstruction<'a> {
    /// Decodes instructions in bytecode order.
    pub(super) fn iter_from_bytes(
        mut bytes: &'a [u8],
    ) -> impl Iterator<Item = Result<(ProgramCounter, Self), ParseError>> {
        let original_len = bytes.len();
        iter::from_fn(move || Self::read_one(&mut bytes, original_len).transpose())
    }

    /// Decodes one instruction using its offset within the complete code array.
    #[allow(clippy::too_many_lines)]
    fn read_one(
        bytes: &mut &'a [u8],
        original_len: usize,
    ) -> Result<Option<(ProgramCounter, Self)>, ParseError> {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let pc = u16::try_from(original_len - bytes.len())
            .map_err(|_| ParseError::malform("The instruction list is too long"))?
            .into();
        if bytes.is_empty() {
            return Ok(None);
        }
        let opcode: u8 = decode_value(bytes)?;
        let instruction = match opcode {
            0x32 => AALoad,
            0x53 => AAStore,
            0x01 => AConstNull,
            0x19 => ALoad {
                index: decode_value(bytes)?,
            },
            0x2a => ALoad0,
            0x2b => ALoad1,
            0x2c => ALoad2,
            0x2d => ALoad3,
            0xbd => ANewArray {
                index: decode_value(bytes)?,
            },
            0xb0 => AReturn,
            0xbe => ArrayLength,
            0x3a => AStore {
                index: decode_value(bytes)?,
            },
            0x4b => AStore0,
            0x4c => AStore1,
            0x4d => AStore2,
            0x4e => AStore3,
            0xbf => AThrow,
            0x33 => BALoad,
            0x54 => BAStore,
            0x10 => BiPush {
                value: decode_value(bytes)?,
            },
            0x34 => CALoad,
            0x55 => CAStore,
            0xc0 => CheckCast {
                target_type_index: decode_value(bytes)?,
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
                index: decode_value(bytes)?,
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
                index: decode_value(bytes)?,
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
                index: decode_value(bytes)?,
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
                index: decode_value(bytes)?,
            },
            0x43 => FStore0,
            0x44 => FStore1,
            0x45 => FStore2,
            0x46 => FStore3,
            0x66 => FSub,
            0xb4 => GetField {
                field_ref_index: decode_value(bytes)?,
            },
            0xb2 => GetStatic {
                field_ref_index: decode_value(bytes)?,
            },
            0xa7 => Goto {
                offset: decode_value(bytes)?,
            },
            0xc8 => GotoW {
                offset: decode_value(bytes)?,
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
                offset: decode_value(bytes)?,
            },
            0xa6 => IfACmpNe {
                offset: decode_value(bytes)?,
            },
            0x9f => IfICmpEq {
                offset: decode_value(bytes)?,
            },
            0xa0 => IfICmpNe {
                offset: decode_value(bytes)?,
            },
            0xa1 => IfICmpLt {
                offset: decode_value(bytes)?,
            },
            0xa2 => IfICmpGe {
                offset: decode_value(bytes)?,
            },
            0xa3 => IfICmpGt {
                offset: decode_value(bytes)?,
            },
            0xa4 => IfICmpLe {
                offset: decode_value(bytes)?,
            },
            0x99 => IfEq {
                offset: decode_value(bytes)?,
            },
            0x9a => IfNe {
                offset: decode_value(bytes)?,
            },
            0x9b => IfLt {
                offset: decode_value(bytes)?,
            },
            0x9c => IfGe {
                offset: decode_value(bytes)?,
            },
            0x9d => IfGt {
                offset: decode_value(bytes)?,
            },
            0x9e => IfLe {
                offset: decode_value(bytes)?,
            },
            0xc7 => IfNonNull {
                offset: decode_value(bytes)?,
            },
            0xc6 => IfNull {
                offset: decode_value(bytes)?,
            },
            0x84 => IInc {
                index: decode_value(bytes)?,
                constant: decode_value(bytes)?,
            },
            0x15 => ILoad {
                index: decode_value(bytes)?,
            },
            0x1a => ILoad0,
            0x1b => ILoad1,
            0x1c => ILoad2,
            0x1d => ILoad3,
            0x68 => IMul,
            0x74 => INeg,
            0xc1 => InstanceOf {
                target_type_index: decode_value(bytes)?,
            },
            0xba => {
                let dynamic_index = decode_value(bytes)?;
                let zero: u16 = decode_value(bytes)?;
                if zero != 0 {
                    Err(ParseError::malform("Zero paddings are not zero"))?;
                }
                InvokeDynamic { dynamic_index }
            }
            0xb9 => {
                let method_index = decode_value(bytes)?;
                let count: u8 = decode_value(bytes)?;
                let zero: u8 = decode_value(bytes)?;
                if zero != 0 {
                    Err(ParseError::malform("Zero paddings are not zero"))?;
                }
                InvokeInterface {
                    method_index,
                    count,
                }
            }
            0xb7 => InvokeSpecial {
                method_index: decode_value(bytes)?,
            },
            0xb8 => InvokeStatic {
                method_index: decode_value(bytes)?,
            },
            0xb6 => InvokeVirtual {
                method_index: decode_value(bytes)?,
            },
            0x80 => IOr,
            0x70 => IRem,
            0xac => IReturn,
            0x78 => IShl,
            0x7a => IShr,
            0x36 => IStore {
                index: decode_value(bytes)?,
            },
            0x3b => IStore0,
            0x3c => IStore1,
            0x3d => IStore2,
            0x3e => IStore3,
            0x64 => ISub,
            0x7c => IUShr,
            0x82 => IXor,
            0xa8 => Jsr {
                offset: decode_value(bytes)?,
            },
            0xc9 => JsrW {
                offset: decode_value(bytes)?,
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
                const_index: decode_value(bytes)?,
            },
            0x13 => LdcW {
                const_index: decode_value(bytes)?,
            },
            0x14 => Ldc2W {
                const_index: decode_value(bytes)?,
            },
            0x6d => LDiv,
            0x16 => LLoad {
                index: decode_value(bytes)?,
            },
            0x1e => LLoad0,
            0x1f => LLoad1,
            0x20 => LLoad2,
            0x21 => LLoad3,
            0x69 => LMul,
            0x75 => LNeg,
            0xab => {
                while !(original_len - bytes.len()).is_multiple_of(4) {
                    let _: u8 = decode_value(bytes)?;
                }
                LookupSwitch(decode_value(bytes)?)
            }
            0xaa => {
                while !(original_len - bytes.len()).is_multiple_of(4) {
                    let _: u8 = decode_value(bytes)?;
                }
                TableSwitch(decode_value(bytes)?)
            }
            0x81 => LOr,
            0x71 => LRem,
            0xad => LReturn,
            0x79 => LShl,
            0x7b => LShr,
            0x37 => LStore {
                index: decode_value(bytes)?,
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
            0xca => Breakpoint,
            0xfe => ImpDep1,
            0xff => ImpDep2,
            0xc5 => MultiANewArray {
                index: decode_value(bytes)?,
                dimensions: decode_value(bytes)?,
            },
            0xbb => New {
                index: decode_value(bytes)?,
            },
            0xbc => NewArray {
                atype: decode_value(bytes)?,
            },
            0x00 => Nop,
            0x57 => Pop,
            0x58 => Pop2,
            0xb5 => PutField {
                field_ref_index: decode_value(bytes)?,
            },
            0xb3 => PutStatic {
                field_ref_index: decode_value(bytes)?,
            },
            0xa9 => Ret {
                index: decode_value(bytes)?,
            },
            0xb1 => Return,
            0x35 => SALoad,
            0x56 => SAStore,
            0x11 => SiPush {
                value: decode_value(bytes)?,
            },
            0x5f => Swap,
            0xc4 => {
                let wide_opcode: u8 = decode_value(bytes)?;
                let wide_insn = match wide_opcode {
                    0x15 => RawWideInstruction::ILoad {
                        index: decode_value(bytes)?,
                    },
                    0x16 => RawWideInstruction::LLoad {
                        index: decode_value(bytes)?,
                    },
                    0x17 => RawWideInstruction::FLoad {
                        index: decode_value(bytes)?,
                    },
                    0x18 => RawWideInstruction::DLoad {
                        index: decode_value(bytes)?,
                    },
                    0x19 => RawWideInstruction::ALoad {
                        index: decode_value(bytes)?,
                    },
                    0x36 => RawWideInstruction::IStore {
                        index: decode_value(bytes)?,
                    },
                    0x37 => RawWideInstruction::LStore {
                        index: decode_value(bytes)?,
                    },
                    0x38 => RawWideInstruction::FStore {
                        index: decode_value(bytes)?,
                    },
                    0x39 => RawWideInstruction::DStore {
                        index: decode_value(bytes)?,
                    },
                    0x3a => RawWideInstruction::AStore {
                        index: decode_value(bytes)?,
                    },
                    0x84 => RawWideInstruction::IInc {
                        index: decode_value(bytes)?,
                        increment: decode_value(bytes)?,
                    },
                    0xa9 => RawWideInstruction::Ret {
                        index: decode_value(bytes)?,
                    },
                    _ => Err(ParseError::malform("Invalid opcode"))?,
                };
                Wide(wide_insn)
            }
            _ => Err(ParseError::malform("Invalid opcode"))?,
        };
        Ok(Some((pc, instruction)))
    }
}

#[cfg(test)]
mod tests;
