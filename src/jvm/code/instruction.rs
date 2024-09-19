use std::{collections::BTreeMap, ops::RangeInclusive, ptr};

use crate::{
    jvm::{
        references::{ClassRef, FieldRef, MethodRef},
        ConstantValue,
    },
    macros::see_jvm_spec,
    types::{
        field_type::{FieldType, PrimitiveType},
        method_descriptor::MethodDescriptor,
    },
};

use super::ProgramCounter;

/// A JVM instruction.
#[doc = see_jvm_spec!(6, 5)]
#[derive(Debug, PartialEq, Clone)]
#[allow(missing_docs)]
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
    ANewArray(ClassRef) = 0xbd,
    ArrayLength = 0xbe,
    AThrow = 0xbf,
    CheckCast(FieldType) = 0xc0,
    InstanceOf(FieldType) = 0xc1,
    MonitorEnter = 0xc2,
    MonitorExit = 0xc3,

    // Extended
    #[deprecated(note = "Use the inner instruction instead")]
    Wide(WideInstruction) = 0xc4,
    MultiANewArray(FieldType, u8) = 0xc5,
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
#[allow(missing_docs, clippy::module_name_repetitions)]
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
        self.discriminant()
    }

    const fn discriminant(&self) -> u8 {
        // SAFETY: Because `Self` is marked `repr(u8)`, its layout is a `repr(C)` `union`
        // between `repr(C)` structs, each of which has the `u8` discriminant as its first
        // field, so we can read the discriminant without offsetting the pointer.
        // See https://doc.rust-lang.org/std/mem/fn.discriminant.html#accessing-the-numeric-value-of-the-discriminant
        unsafe { *ptr::from_ref(self).cast::<u8>() }
    }

    /// Gets the name of the [Instruction].
    #[must_use]
    #[allow(clippy::too_many_lines)]
    pub const fn name<'a>(&self) -> &'a str {
        #[allow(clippy::enum_glob_use)]
        use Instruction::*;

        match self {
            AALoad => "aaload",
            AAStore => "aastore",
            AConstNull => "aconst_null",
            ALoad(_) => "aload",
            ALoad0 => "aload_0",
            ALoad1 => "aload_1",
            ALoad2 => "aload_2",
            ALoad3 => "aload_3",
            ANewArray(_) => "anewarray",
            AReturn => "areturn",
            ArrayLength => "arraylength",
            AStore(_) => "astore",
            AStore0 => "astore_0",
            AStore1 => "astore_1",
            AStore2 => "astore_2",
            AStore3 => "astore_3",
            AThrow => "athrow",
            BALoad => "baload",
            BAStore => "bastore",
            BiPush(_) => "bipush",
            CALoad => "caload",
            CAStore => "castore",
            CheckCast(_) => "checkcast",
            D2F => "d2f",
            D2I => "d2i",
            D2L => "d2l",
            DAdd => "dadd",
            DALoad => "daload",
            DAStore => "dastore",
            DCmpG => "dcmpg",
            DCmpL => "dcmpl",
            DConst0 => "dconst_0",
            DConst1 => "dconst_1",
            DDiv => "ddiv",
            DLoad(_) => "dload",
            DLoad0 => "dload_0",
            DLoad1 => "dload_1",
            DLoad2 => "dload_2",
            DLoad3 => "dload_3",
            DMul => "dmul",
            DNeg => "dneg",
            DRem => "drem",
            DReturn => "dreturn",
            DStore(_) => "dstore",
            DStore0 => "dstore_0",
            DStore1 => "dstore_1",
            DStore2 => "dstore_2",
            DStore3 => "dstore_3",
            DSub => "dsub",
            Dup => "dup",
            DupX1 => "dup_x1",
            DupX2 => "dup_x2",
            Dup2 => "dup2",
            Dup2X1 => "dup2_x1",
            Dup2X2 => "dup2_x2",
            F2D => "f2d",
            F2I => "f2i",
            F2L => "f2l",
            FAdd => "fadd",
            FALoad => "faload",
            FAStore => "fastore",
            FCmpG => "fcmpg",
            FCmpL => "fcmpl",
            FConst0 => "fconst_0",
            FConst1 => "fconst_1",
            FConst2 => "fconst_2",
            FDiv => "fdiv",
            FLoad(_) => "fload",
            FLoad0 => "fload_0",
            FLoad1 => "fload_1",
            FLoad2 => "fload_2",
            FLoad3 => "fload_3",
            FMul => "fmul",
            FNeg => "fneg",
            FRem => "frem",
            FReturn => "freturn",
            FStore(_) => "fstore",
            FStore0 => "fstore_0",
            FStore1 => "fstore_1",
            FStore2 => "fstore_2",
            FStore3 => "fstore_3",
            FSub => "fsub",
            GetField(_) => "getfield",
            GetStatic(_) => "getstatic",
            Goto(_) => "goto",
            GotoW(_) => "goto_w",
            I2B => "i2b",
            I2C => "i2c",
            I2D => "i2d",
            I2F => "i2f",
            I2L => "i2l",
            I2S => "i2s",
            IAdd => "iadd",
            IALoad => "iaload",
            IAnd => "iand",
            IAStore => "iastore",
            IConstM1 => "iconst_m1",
            IConst0 => "iconst_0",
            IConst1 => "iconst_1",
            IConst2 => "iconst_2",
            IConst3 => "iconst_3",
            IConst4 => "iconst_4",
            IConst5 => "iconst_5",
            IDiv => "idiv",
            IfACmpEq(_) => "if_acmpeq",
            IfACmpNe(_) => "if_acmpne",
            IfICmpEq(_) => "if_icmpeq",
            IfICmpNe(_) => "if_icmpne",
            IfICmpLt(_) => "if_icmplt",
            IfICmpGe(_) => "if_icmpge",
            IfICmpGt(_) => "if_icmpgt",
            IfICmpLe(_) => "if_icmple",
            IfEq(_) => "ifeq",
            IfNe(_) => "ifne",
            IfLt(_) => "iflt",
            IfGe(_) => "ifge",
            IfGt(_) => "ifgt",
            IfLe(_) => "ifle",
            IfNonNull(_) => "ifnonnull",
            IfNull(_) => "ifnull",
            IInc(_, _) => "iinc",
            ILoad(_) => "iload",
            ILoad0 => "iload_0",
            ILoad1 => "iload_1",
            ILoad2 => "iload_2",
            ILoad3 => "iload_3",
            IMul => "imul",
            INeg => "ineg",
            InstanceOf(_) => "instanceof",
            InvokeDynamic { .. } => "invokedynamic",
            InvokeInterface(_, _) => "invokeinterface",
            InvokeSpecial(_) => "invokespecial",
            InvokeStatic(_) => "invokestatic",
            InvokeVirtual(_) => "invokevirtual",
            IOr => "ior",
            IRem => "irem",
            IReturn => "ireturn",
            IShl => "ishl",
            IShr => "ishr",
            IStore(_) => "istore",
            IStore0 => "istore_0",
            IStore1 => "istore_1",
            IStore2 => "istore_2",
            IStore3 => "istore_3",
            ISub => "isub",
            IUShr => "iushr",
            IXor => "ixor",
            Jsr(_) => "jsr",
            JsrW(_) => "jsr_w",
            L2D => "l2d",
            L2F => "l2f",
            L2I => "l2i",
            LAdd => "ladd",
            LALoad => "laload",
            LAnd => "land",
            LAStore => "lastore",
            LCmp => "lcmp",
            LConst0 => "lconst_0",
            LConst1 => "lconst_1",
            Ldc(_) => "ldc",
            LdcW(_) => "ldc_w",
            Ldc2W(_) => "ldc2_w",
            LDiv => "ldiv",
            LLoad(_) => "lload",
            LLoad0 => "lload_0",
            LLoad1 => "lload_1",
            LLoad2 => "lload_2",
            LLoad3 => "lload_3",
            LMul => "lmul",
            LNeg => "lneg",
            LookupSwitch { .. } => "lookupswitch",
            TableSwitch { .. } => "tableswitch",
            LOr => "lor",
            LRem => "lrem",
            LReturn => "lreturn",
            LShl => "lshl",
            LShr => "lshr",
            LStore(_) => "lstore",
            LStore0 => "lstore_0",
            LStore1 => "lstore_1",
            LStore2 => "lstore_2",
            LStore3 => "lstore_3",
            LSub => "lsub",
            LUShr => "lushr",
            LXor => "lxor",
            MonitorEnter => "monitorenter",
            MonitorExit => "monitorexit",
            MultiANewArray(_, _) => "multianewarray",
            New(_) => "new",
            NewArray(_) => "newarray",
            Nop => "nop",
            Pop => "pop",
            Pop2 => "pop2",
            PutField(_) => "putfield",
            PutStatic(_) => "putstatic",
            Ret(_) => "ret",
            Return => "return",
            SALoad => "saload",
            SAStore => "sastore",
            SiPush(_) => "sipush",
            Swap => "swap",
            Wide(_) => "wide",
            Breakpoint => "breakpoint",
            ImpDep1 => "impdep1",
            ImpDep2 => "impdep2",
        }
    }
}

#[cfg(test)]
mod test {
    use super::Instruction::*;

    #[test]
    fn test_opcode() {
        assert_eq!(Nop.opcode(), 0x00);
        assert_eq!(AConstNull.opcode(), 0x01);
        assert_eq!(IConstM1.opcode(), 0x02);
        assert_eq!(ILoad(233).opcode(), 0x15);
    }
}
