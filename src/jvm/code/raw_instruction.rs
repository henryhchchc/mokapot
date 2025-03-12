use crate::{macros::see_jvm_spec, utils::enum_discriminant};

/// A raw JVM instruction without the information form the constant pool.
#[doc = see_jvm_spec!(6, 5)]
#[repr(u8)]
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Clone)]
pub enum RawInstruction {
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
    BiPush {
        value: u8,
    } = 0x10,
    SiPush {
        value: u16,
    } = 0x11,
    Ldc {
        const_index: u8,
    } = 0x12,
    LdcW {
        const_index: u16,
    } = 0x13,
    Ldc2W {
        const_index: u16,
    } = 0x14,
    ILoad {
        index: u8,
    } = 0x15,
    LLoad {
        index: u8,
    } = 0x16,
    FLoad {
        index: u8,
    } = 0x17,
    DLoad {
        index: u8,
    } = 0x18,
    ALoad {
        index: u8,
    } = 0x19,
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
    IStore {
        index: u8,
    } = 0x36,
    LStore {
        index: u8,
    } = 0x37,
    FStore {
        index: u8,
    } = 0x38,
    DStore {
        index: u8,
    } = 0x39,
    AStore {
        index: u8,
    } = 0x3A,
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
    IInc {
        index: u8,
        constant: i8,
    } = 0x84,
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
    IfEq {
        offset: i16,
    } = 0x99,
    IfNe {
        offset: i16,
    } = 0x9A,
    IfLt {
        offset: i16,
    } = 0x9B,
    IfGe {
        offset: i16,
    } = 0x9C,
    IfGt {
        offset: i16,
    } = 0x9D,
    IfLe {
        offset: i16,
    } = 0x9E,
    IfICmpEq {
        offset: i16,
    } = 0x9F,
    IfICmpNe {
        offset: i16,
    } = 0xA0,
    IfICmpLt {
        offset: i16,
    } = 0xA1,
    IfICmpGe {
        offset: i16,
    } = 0xA2,
    IfICmpGt {
        offset: i16,
    } = 0xA3,
    IfICmpLe {
        offset: i16,
    } = 0xA4,
    IfACmpEq {
        offset: i16,
    } = 0xA5,
    IfACmpNe {
        offset: i16,
    } = 0xA6,
    Goto {
        offset: i16,
    } = 0xA7,
    Jsr {
        offset: i16,
    } = 0xA8,
    Ret {
        index: u8,
    } = 0xA9,
    TableSwitch {
        default: i32,
        low: i32,
        high: i32,
        jump_offsets: Vec<i32>,
    } = 0xAA,
    LookupSwitch {
        default: i32,
        match_offsets: Vec<(i32, i32)>,
    } = 0xAB,
    IReturn = 0xAC,
    LReturn = 0xAD,
    FReturn = 0xAE,
    DReturn = 0xAF,
    AReturn = 0xB0,
    Return = 0xB1,
    GetStatic {
        field_ref_index: u16,
    } = 0xB2,
    PutStatic {
        field_ref_index: u16,
    } = 0xB3,
    GetField {
        field_ref_index: u16,
    } = 0xB4,
    PutField {
        field_ref_index: u16,
    } = 0xB5,
    InvokeVirtual {
        method_index: u16,
    } = 0xB6,
    InvokeSpecial {
        method_index: u16,
    } = 0xB7,
    InvokeStatic {
        method_index: u16,
    } = 0xB8,
    InvokeInterface {
        method_index: u16,
        count: u8,
    } = 0xB9,
    InvokeDynamic {
        dynamic_index: u16,
    } = 0xBA,
    New {
        index: u16,
    } = 0xBB,
    NewArray {
        atype: u8,
    } = 0xBC,
    ANewArray {
        index: u16,
    } = 0xBD,
    ArrayLength = 0xBE,
    AThrow = 0xBF,
    CheckCast {
        target_type_index: u16,
    } = 0xC0,
    InstanceOf {
        target_type_index: u16,
    } = 0xC1,
    MonitorEnter = 0xC2,
    MonitorExit = 0xC3,
    Wide(RawWideInstruction) = 0xC4,
    MultiANewArray {
        index: u16,
        dimensions: u8,
    } = 0xC5,
    IfNull {
        offset: i16,
    } = 0xC6,
    IfNonNull {
        offset: i16,
    } = 0xC7,
    GotoW {
        offset: i32,
    } = 0xC8,
    JsrW {
        offset: i32,
    } = 0xC9,
    Breakpoint = 0xCA,
    ImpDep1 = 0xFE,
    ImpDep2 = 0xFF,
}

/// A wide instruction.
#[allow(missing_docs)]
#[derive(Debug, PartialEq, Eq, Clone)]
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

impl RawInstruction {
    /// Gets the opcode.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        // Safery: Self is repr(u8) so it should be fine
        unsafe { enum_discriminant(self) }
    }
}

impl RawWideInstruction {
    /// Gets the opcode.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        // Safery: Self is repr(u8) so it should be fine
        unsafe { enum_discriminant(self) }
    }
}

#[cfg(test)]
mod test {
    use super::RawInstruction::*;

    #[test]
    fn test_opcode() {
        assert_eq!(Nop.opcode(), 0x00);
        assert_eq!(AConstNull.opcode(), 0x01);
        assert_eq!(IConstM1.opcode(), 0x02);
        assert_eq!(ILoad { index: 233 }.opcode(), 0x15);
    }
}
