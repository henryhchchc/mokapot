//! Resolved JVM instruction model and presentation.

mod display;
mod name;

use std::{collections::BTreeMap, ops::RangeInclusive};

use super::ProgramCounter;
use crate::{
    intrinsics::{enum_discriminant, see_jvm_spec},
    jvm::{
        ConstantValue,
        references::{ClassRef, FieldRef, MethodRef},
    },
    types::{
        field_type::PrimitiveType, method_descriptor::MethodDescriptor,
        reference_type::ReferenceType,
    },
};

/// A JVM instruction.
#[doc = see_jvm_spec!(6, 5)]
#[derive(Debug, PartialEq, Clone)]
#[allow(
    missing_docs,
    reason = "This maps one-to-one to those in the JVM spec."
)]
#[repr(u8)]
pub enum Instruction {
    // Constants
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
    LConst1 = 0x0a,
    FConst0 = 0x0b,
    FConst1 = 0x0c,
    FConst2 = 0x0d,
    DConst0 = 0x0e,
    DConst1 = 0x0f,
    BiPush(u8) = 0x10,
    SiPush(u16) = 0x11,
    Ldc(ConstantValue) = 0x12,
    LdcW(ConstantValue) = 0x13,
    Ldc2W(ConstantValue) = 0x14,

    // Loads
    ILoad(u8) = 0x15,
    LLoad(u8) = 0x16,
    FLoad(u8) = 0x17,
    DLoad(u8) = 0x18,
    ALoad(u8) = 0x19,
    ILoad0 = 0x1a,
    ILoad1 = 0x1b,
    ILoad2 = 0x1c,
    ILoad3 = 0x1d,
    LLoad0 = 0x1e,
    LLoad1 = 0x1f,
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
    ALoad0 = 0x2a,
    ALoad1 = 0x2b,
    ALoad2 = 0x2c,
    ALoad3 = 0x2d,
    IALoad = 0x2e,
    LALoad = 0x2f,
    FALoad = 0x30,
    DALoad = 0x31,
    AALoad = 0x32,
    BALoad = 0x33,
    CALoad = 0x34,
    SALoad = 0x35,

    // Stores
    IStore(u8) = 0x36,
    LStore(u8) = 0x37,
    FStore(u8) = 0x38,
    DStore(u8) = 0x39,
    AStore(u8) = 0x3a,
    IStore0 = 0x3b,
    IStore1 = 0x3c,
    IStore2 = 0x3d,
    IStore3 = 0x3e,
    LStore0 = 0x3f,
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
    DStore3 = 0x4a,
    AStore0 = 0x4b,
    AStore1 = 0x4c,
    AStore2 = 0x4d,
    AStore3 = 0x4e,
    IAStore = 0x4f,
    LAStore = 0x50,
    FAStore = 0x51,
    DAStore = 0x52,
    AAStore = 0x53,
    BAStore = 0x54,
    CAStore = 0x55,
    SAStore = 0x56,

    // Stack
    Pop = 0x57,
    Pop2 = 0x58,
    Dup = 0x59,
    DupX1 = 0x5a,
    DupX2 = 0x5b,
    Dup2 = 0x5c,
    Dup2X1 = 0x5d,
    Dup2X2 = 0x5e,
    Swap = 0x5f,

    // Math
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
    FMul = 0x6a,
    DMul = 0x6b,
    IDiv = 0x6c,
    LDiv = 0x6d,
    FDiv = 0x6e,
    DDiv = 0x6f,
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
    IShr = 0x7a,
    LShr = 0x7b,
    IUShr = 0x7c,
    LUShr = 0x7d,
    IAnd = 0x7e,
    LAnd = 0x7f,
    IOr = 0x80,
    LOr = 0x81,
    IXor = 0x82,
    LXor = 0x83,
    IInc(u8, i32) = 0x84,

    // Conversions
    I2L = 0x85,
    I2F = 0x86,
    I2D = 0x87,
    L2I = 0x88,
    L2F = 0x89,
    L2D = 0x8a,
    F2I = 0x8b,
    F2L = 0x8c,
    F2D = 0x8d,
    D2I = 0x8e,
    D2L = 0x8f,
    D2F = 0x90,
    I2B = 0x91,
    I2C = 0x92,
    I2S = 0x93,

    // Comparisons
    LCmp = 0x94,
    FCmpL = 0x95,
    FCmpG = 0x96,
    DCmpL = 0x97,
    DCmpG = 0x98,
    IfEq(ProgramCounter) = 0x99,
    IfNe(ProgramCounter) = 0x9a,
    IfLt(ProgramCounter) = 0x9b,
    IfGe(ProgramCounter) = 0x9c,
    IfGt(ProgramCounter) = 0x9d,
    IfLe(ProgramCounter) = 0x9e,
    IfICmpEq(ProgramCounter) = 0x9f,
    IfICmpNe(ProgramCounter) = 0xa0,
    IfICmpLt(ProgramCounter) = 0xa1,
    IfICmpGe(ProgramCounter) = 0xa2,
    IfICmpGt(ProgramCounter) = 0xa3,
    IfICmpLe(ProgramCounter) = 0xa4,
    IfACmpEq(ProgramCounter) = 0xa5,
    IfACmpNe(ProgramCounter) = 0xa6,

    // Control
    Goto(ProgramCounter) = 0xa7,
    Jsr(ProgramCounter) = 0xa8,
    Ret(u8) = 0xa9,
    TableSwitch {
        range: RangeInclusive<i32>,
        jump_targets: Vec<ProgramCounter>,
        default: ProgramCounter,
    } = 0xaa,
    LookupSwitch {
        default: ProgramCounter,
        match_targets: BTreeMap<i32, ProgramCounter>,
    } = 0xab,
    IReturn = 0xac,
    LReturn = 0xad,
    FReturn = 0xae,
    DReturn = 0xaf,
    AReturn = 0xb0,
    Return = 0xb1,

    // References
    GetStatic(FieldRef) = 0xb2,
    PutStatic(FieldRef) = 0xb3,
    GetField(FieldRef) = 0xb4,
    PutField(FieldRef) = 0xb5,
    InvokeVirtual(MethodRef) = 0xb6,
    InvokeSpecial(MethodRef) = 0xb7,
    InvokeStatic(MethodRef) = 0xb8,
    InvokeInterface(MethodRef, u8) = 0xb9,
    InvokeDynamic {
        bootstrap_method_index: u16,
        name: String,
        descriptor: MethodDescriptor,
    } = 0xba,
    New(ClassRef) = 0xbb,
    NewArray(PrimitiveType) = 0xbc,
    ANewArray(ReferenceType) = 0xbd,
    ArrayLength = 0xbe,
    AThrow = 0xbf,
    CheckCast(ReferenceType) = 0xc0,
    InstanceOf(ReferenceType) = 0xc1,
    MonitorEnter = 0xc2,
    MonitorExit = 0xc3,

    // Extended
    Wide(WideInstruction) = 0xc4,
    MultiANewArray(ReferenceType, u8) = 0xc5,
    IfNull(ProgramCounter) = 0xc6,
    IfNonNull(ProgramCounter) = 0xc7,
    GotoW(ProgramCounter) = 0xc8,
    JsrW(ProgramCounter) = 0xc9,

    // Reserved
    Breakpoint = 0xca,
    ImpDep1 = 0xfe,
    ImpDep2 = 0xff,
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
#[derive(Debug, PartialEq, Eq, Clone)]
pub enum WideInstruction {
    ILoad(u16),
    LLoad(u16),
    FLoad(u16),
    DLoad(u16),
    ALoad(u16),
    IStore(u16),
    LStore(u16),
    FStore(u16),
    DStore(u16),
    AStore(u16),
    IInc(u16, i32),
    Ret(u16),
}

impl Instruction {
    /// Gets the opcode.
    #[must_use]
    pub const fn opcode(&self) -> u8 {
        // SAFETY: Self is repr(u8) so it should be fine
        unsafe { enum_discriminant(self) }
    }
}

#[cfg(test)]
mod tests;
