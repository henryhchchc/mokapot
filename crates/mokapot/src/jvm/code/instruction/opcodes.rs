//! Shared opcodes, mnemonics, operand types, and borrowed-view conversions.
// The consumer generates complete items because macros cannot expand to enum variants.
macro_rules! for_each_instruction {
    ($emit:ident, $class:lifetime, $pool:ident) => {
        $emit! {
            $class, $pool;
            unit {
                Nop = 0x00, "nop";
                AConstNull = 0x01, "aconst_null";
                IConstM1 = 0x02, "iconst_m1";
                IConst0 = 0x03, "iconst_0";
                IConst1 = 0x04, "iconst_1";
                IConst2 = 0x05, "iconst_2";
                IConst3 = 0x06, "iconst_3";
                IConst4 = 0x07, "iconst_4";
                IConst5 = 0x08, "iconst_5";
                LConst0 = 0x09, "lconst_0";
                LConst1 = 0x0a, "lconst_1";
                FConst0 = 0x0b, "fconst_0";
                FConst1 = 0x0c, "fconst_1";
                FConst2 = 0x0d, "fconst_2";
                DConst0 = 0x0e, "dconst_0";
                DConst1 = 0x0f, "dconst_1";
                ILoad0 = 0x1a, "iload_0";
                ILoad1 = 0x1b, "iload_1";
                ILoad2 = 0x1c, "iload_2";
                ILoad3 = 0x1d, "iload_3";
                LLoad0 = 0x1e, "lload_0";
                LLoad1 = 0x1f, "lload_1";
                LLoad2 = 0x20, "lload_2";
                LLoad3 = 0x21, "lload_3";
                FLoad0 = 0x22, "fload_0";
                FLoad1 = 0x23, "fload_1";
                FLoad2 = 0x24, "fload_2";
                FLoad3 = 0x25, "fload_3";
                DLoad0 = 0x26, "dload_0";
                DLoad1 = 0x27, "dload_1";
                DLoad2 = 0x28, "dload_2";
                DLoad3 = 0x29, "dload_3";
                ALoad0 = 0x2a, "aload_0";
                ALoad1 = 0x2b, "aload_1";
                ALoad2 = 0x2c, "aload_2";
                ALoad3 = 0x2d, "aload_3";
                IALoad = 0x2e, "iaload";
                LALoad = 0x2f, "laload";
                FALoad = 0x30, "faload";
                DALoad = 0x31, "daload";
                AALoad = 0x32, "aaload";
                BALoad = 0x33, "baload";
                CALoad = 0x34, "caload";
                SALoad = 0x35, "saload";
                IStore0 = 0x3b, "istore_0";
                IStore1 = 0x3c, "istore_1";
                IStore2 = 0x3d, "istore_2";
                IStore3 = 0x3e, "istore_3";
                LStore0 = 0x3f, "lstore_0";
                LStore1 = 0x40, "lstore_1";
                LStore2 = 0x41, "lstore_2";
                LStore3 = 0x42, "lstore_3";
                FStore0 = 0x43, "fstore_0";
                FStore1 = 0x44, "fstore_1";
                FStore2 = 0x45, "fstore_2";
                FStore3 = 0x46, "fstore_3";
                DStore0 = 0x47, "dstore_0";
                DStore1 = 0x48, "dstore_1";
                DStore2 = 0x49, "dstore_2";
                DStore3 = 0x4a, "dstore_3";
                AStore0 = 0x4b, "astore_0";
                AStore1 = 0x4c, "astore_1";
                AStore2 = 0x4d, "astore_2";
                AStore3 = 0x4e, "astore_3";
                IAStore = 0x4f, "iastore";
                LAStore = 0x50, "lastore";
                FAStore = 0x51, "fastore";
                DAStore = 0x52, "dastore";
                AAStore = 0x53, "aastore";
                BAStore = 0x54, "bastore";
                CAStore = 0x55, "castore";
                SAStore = 0x56, "sastore";
                Pop = 0x57, "pop";
                Pop2 = 0x58, "pop2";
                Dup = 0x59, "dup";
                DupX1 = 0x5a, "dup_x1";
                DupX2 = 0x5b, "dup_x2";
                Dup2 = 0x5c, "dup2";
                Dup2X1 = 0x5d, "dup2_x1";
                Dup2X2 = 0x5e, "dup2_x2";
                Swap = 0x5f, "swap";
                IAdd = 0x60, "iadd";
                LAdd = 0x61, "ladd";
                FAdd = 0x62, "fadd";
                DAdd = 0x63, "dadd";
                ISub = 0x64, "isub";
                LSub = 0x65, "lsub";
                FSub = 0x66, "fsub";
                DSub = 0x67, "dsub";
                IMul = 0x68, "imul";
                LMul = 0x69, "lmul";
                FMul = 0x6a, "fmul";
                DMul = 0x6b, "dmul";
                IDiv = 0x6c, "idiv";
                LDiv = 0x6d, "ldiv";
                FDiv = 0x6e, "fdiv";
                DDiv = 0x6f, "ddiv";
                IRem = 0x70, "irem";
                LRem = 0x71, "lrem";
                FRem = 0x72, "frem";
                DRem = 0x73, "drem";
                INeg = 0x74, "ineg";
                LNeg = 0x75, "lneg";
                FNeg = 0x76, "fneg";
                DNeg = 0x77, "dneg";
                IShl = 0x78, "ishl";
                LShl = 0x79, "lshl";
                IShr = 0x7a, "ishr";
                LShr = 0x7b, "lshr";
                IUShr = 0x7c, "iushr";
                LUShr = 0x7d, "lushr";
                IAnd = 0x7e, "iand";
                LAnd = 0x7f, "land";
                IOr = 0x80, "ior";
                LOr = 0x81, "lor";
                IXor = 0x82, "ixor";
                LXor = 0x83, "lxor";
                I2L = 0x85, "i2l";
                I2F = 0x86, "i2f";
                I2D = 0x87, "i2d";
                L2I = 0x88, "l2i";
                L2F = 0x89, "l2f";
                L2D = 0x8a, "l2d";
                F2I = 0x8b, "f2i";
                F2L = 0x8c, "f2l";
                F2D = 0x8d, "f2d";
                D2I = 0x8e, "d2i";
                D2L = 0x8f, "d2l";
                D2F = 0x90, "d2f";
                I2B = 0x91, "i2b";
                I2C = 0x92, "i2c";
                I2S = 0x93, "i2s";
                LCmp = 0x94, "lcmp";
                FCmpL = 0x95, "fcmpl";
                FCmpG = 0x96, "fcmpg";
                DCmpL = 0x97, "dcmpl";
                DCmpG = 0x98, "dcmpg";
                IReturn = 0xac, "ireturn";
                LReturn = 0xad, "lreturn";
                FReturn = 0xae, "freturn";
                DReturn = 0xaf, "dreturn";
                AReturn = 0xb0, "areturn";
                Return = 0xb1, "return";
                ArrayLength = 0xbe, "arraylength";
                AThrow = 0xbf, "athrow";
                MonitorEnter = 0xc2, "monitorenter";
                MonitorExit = 0xc3, "monitorexit";
                Breakpoint = 0xca, "breakpoint";
                ImpDep1 = 0xfe, "impdep1";
                ImpDep2 = 0xff, "impdep2";
            }
            copy {
                BiPush(value0: i8) = 0x10, "bipush";
                SiPush(value0: i16) = 0x11, "sipush";
                ILoad(value0: u8) = 0x15, "iload";
                LLoad(value0: u8) = 0x16, "lload";
                FLoad(value0: u8) = 0x17, "fload";
                DLoad(value0: u8) = 0x18, "dload";
                ALoad(value0: u8) = 0x19, "aload";
                IStore(value0: u8) = 0x36, "istore";
                LStore(value0: u8) = 0x37, "lstore";
                FStore(value0: u8) = 0x38, "fstore";
                DStore(value0: u8) = 0x39, "dstore";
                AStore(value0: u8) = 0x3a, "astore";
                IInc(value0: u8, value1: i32) = 0x84, "iinc";
                IfEq(value0: ProgramCounter) = 0x99, "ifeq";
                IfNe(value0: ProgramCounter) = 0x9a, "ifne";
                IfLt(value0: ProgramCounter) = 0x9b, "iflt";
                IfGe(value0: ProgramCounter) = 0x9c, "ifge";
                IfGt(value0: ProgramCounter) = 0x9d, "ifgt";
                IfLe(value0: ProgramCounter) = 0x9e, "ifle";
                IfICmpEq(value0: ProgramCounter) = 0x9f, "if_icmpeq";
                IfICmpNe(value0: ProgramCounter) = 0xa0, "if_icmpne";
                IfICmpLt(value0: ProgramCounter) = 0xa1, "if_icmplt";
                IfICmpGe(value0: ProgramCounter) = 0xa2, "if_icmpge";
                IfICmpGt(value0: ProgramCounter) = 0xa3, "if_icmpgt";
                IfICmpLe(value0: ProgramCounter) = 0xa4, "if_icmple";
                IfACmpEq(value0: ProgramCounter) = 0xa5, "if_acmpeq";
                IfACmpNe(value0: ProgramCounter) = 0xa6, "if_acmpne";
                Goto(value0: ProgramCounter) = 0xa7, "goto";
                Jsr(value0: ProgramCounter) = 0xa8, "jsr";
                Ret(value0: u8) = 0xa9, "ret";
                NewArray(value0: PrimitiveType) = 0xbc, "newarray";
                Wide(value0: WideInstruction) = 0xc4, "wide";
                IfNull(value0: ProgramCounter) = 0xc6, "ifnull";
                IfNonNull(value0: ProgramCounter) = 0xc7, "ifnonnull";
                GotoW(value0: ProgramCounter) = 0xc8, "goto_w";
                JsrW(value0: ProgramCounter) = 0xc9, "jsr_w";
            }
            pool {
                Ldc(value0: ConstantValueId => ConstantVal<$class> = $pool.constant_value(*value0)) = 0x12, "ldc";
                LdcW(value0: ConstantValueId => ConstantVal<$class> = $pool.constant_value(*value0)) = 0x13, "ldc_w";
                Ldc2W(value0: ConstantValueId => ConstantVal<$class> = $pool.constant_value(*value0)) = 0x14, "ldc2_w";
                GetStatic(value0: FieldRefId => FieldRef<$class> = $pool.field_ref(*value0)) = 0xb2, "getstatic";
                PutStatic(value0: FieldRefId => FieldRef<$class> = $pool.field_ref(*value0)) = 0xb3, "putstatic";
                GetField(value0: FieldRefId => FieldRef<$class> = $pool.field_ref(*value0)) = 0xb4, "getfield";
                PutField(value0: FieldRefId => FieldRef<$class> = $pool.field_ref(*value0)) = 0xb5, "putfield";
                InvokeVirtual(value0: MethodRefId => MethodRef<$class> = $pool.method_ref(*value0)) = 0xb6, "invokevirtual";
                InvokeSpecial(value0: MethodRefId => MethodRef<$class> = $pool.method_ref(*value0)) = 0xb7, "invokespecial";
                InvokeStatic(value0: MethodRefId => MethodRef<$class> = $pool.method_ref(*value0)) = 0xb8, "invokestatic";
                InvokeInterface(value0: MethodRefId => MethodRef<$class> = $pool.method_ref(*value0), value1: u8 => u8 = *value1) = 0xb9, "invokeinterface";
                New(value0: ClassId => &$class ClassName = $pool.class_name(*value0)) = 0xbb, "new";
                ANewArray(value0: ReferenceTypeId => &$class ReferenceType = $pool.reference_type(*value0)) = 0xbd, "anewarray";
                CheckCast(value0: ReferenceTypeId => &$class ReferenceType = $pool.reference_type(*value0)) = 0xc0, "checkcast";
                InstanceOf(value0: ReferenceTypeId => &$class ReferenceType = $pool.reference_type(*value0)) = 0xc1, "instanceof";
                MultiANewArray(value0: ReferenceTypeId => &$class ReferenceType = $pool.reference_type(*value0), value1: u8 => u8 = *value1) = 0xc5, "multianewarray";
            }
            structured {
                TableSwitch {
                    low: i32 => i32 = *low,
                    jump_targets: Box<[ProgramCounter]> => &$class [ProgramCounter] = jump_targets,
                    default: ProgramCounter => ProgramCounter = *default,
                } = 0xaa, "tableswitch";
                LookupSwitch {
                    default: ProgramCounter => ProgramCounter = *default,
                    /// Match values and targets, ordered by strictly increasing match value.
                    match_targets: Box<[(i32, ProgramCounter)]> => &$class [(i32, ProgramCounter)] = match_targets,
                } = 0xab, "lookupswitch";
            }
            custom {
                InvokeDynamic(id: InvokeDynamicId) => {
                    bootstrap_method_index: u16,
                    name: &$class JavaStr,
                    descriptor: &$class MethodDescriptor,
                } = 0xba, "invokedynamic" {
                    let (bootstrap_method_index, name, descriptor) = $pool.invoke_dynamic(*id);
                    Instruction::InvokeDynamic { bootstrap_method_index, name, descriptor }
                }
            }
        }
    };
}

pub(super) use for_each_instruction;
