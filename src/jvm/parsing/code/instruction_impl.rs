use std::{collections::BTreeMap, str::FromStr};

use itertools::Itertools;

use crate::{
    jvm::{
        class::constant_pool::{ConstantPool, ConstantPoolEntry},
        code::{
            Instruction, InstructionList, ProgramCounter, RawInstruction, RawWideInstruction,
            WideInstruction,
        },
        method::MethodDescriptor,
        parsing::parsing_context::ParsingContext,
        ClassFileParsingError, ClassFileParsingResult,
    },
    types::field_type::PrimitiveType,
};

impl Instruction {
    pub(crate) fn parse_code(
        reader: Vec<u8>,
        ctx: &ParsingContext,
    ) -> ClassFileParsingResult<InstructionList<Instruction>> {
        let raw_instructions = RawInstruction::from_bytes(reader)?;
        let inner: BTreeMap<ProgramCounter, Self> = raw_instructions
            .into_iter()
            .map(|(pc, raw_insn)| {
                Self::from_raw_instruction(raw_insn, pc, &ctx.constant_pool).map(|it| (pc, it))
            })
            .collect::<ClassFileParsingResult<_>>()?;
        Ok(InstructionList::from(inner))
    }

    #[allow(clippy::too_many_lines)]
    pub(crate) fn from_raw_instruction(
        raw_instruction: RawInstruction,
        pc: ProgramCounter,
        constant_pool: &ConstantPool,
    ) -> ClassFileParsingResult<Self> {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let result = match raw_instruction {
            // Constants
            Nop => Self::Nop,
            AConstNull => Self::AConstNull,
            IConstM1 => Self::IConstM1,
            IConst0 => Self::IConst0,
            IConst1 => Self::IConst1,
            IConst2 => Self::IConst2,
            IConst3 => Self::IConst3,
            IConst4 => Self::IConst4,
            IConst5 => Self::IConst5,
            LConst0 => Self::LConst0,
            LConst1 => Self::LConst1,
            FConst0 => Self::FConst0,
            FConst1 => Self::FConst1,
            FConst2 => Self::FConst2,
            DConst0 => Self::DConst0,
            DConst1 => Self::DConst1,
            BiPush { value } => Self::BiPush(value),
            SiPush { value } => Self::SiPush(value),
            Ldc { const_index } => {
                let constant = constant_pool.get_constant_value(u16::from(const_index))?;
                Self::Ldc(constant)
            }
            LdcW { const_index } => {
                let constant = constant_pool.get_constant_value(const_index)?;
                Self::LdcW(constant)
            }
            Ldc2W { const_index } => {
                let constant = constant_pool.get_constant_value(const_index)?;
                Self::Ldc2W(constant)
            }

            // Loads
            ILoad { index } => Self::ILoad(index),
            LLoad { index } => Self::LLoad(index),
            FLoad { index } => Self::FLoad(index),
            DLoad { index } => Self::DLoad(index),
            ALoad { index } => Self::ALoad(index),
            ILoad0 => Self::ILoad0,
            ILoad1 => Self::ILoad1,
            ILoad2 => Self::ILoad2,
            ILoad3 => Self::ILoad3,
            LLoad0 => Self::LLoad0,
            LLoad1 => Self::LLoad1,
            LLoad2 => Self::LLoad2,
            LLoad3 => Self::LLoad3,
            FLoad0 => Self::FLoad0,
            FLoad1 => Self::FLoad1,
            FLoad2 => Self::FLoad2,
            FLoad3 => Self::FLoad3,
            DLoad0 => Self::DLoad0,
            DLoad1 => Self::DLoad1,
            DLoad2 => Self::DLoad2,
            DLoad3 => Self::DLoad3,
            ALoad0 => Self::ALoad0,
            ALoad1 => Self::ALoad1,
            ALoad2 => Self::ALoad2,
            ALoad3 => Self::ALoad3,
            IALoad => Self::IALoad,
            LALoad => Self::LALoad,
            FALoad => Self::FALoad,
            DALoad => Self::DALoad,
            AALoad => Self::AALoad,
            BALoad => Self::BALoad,
            CALoad => Self::CALoad,
            SALoad => Self::SALoad,

            // Stores
            IStore { index } => Self::IStore(index),
            LStore { index } => Self::LStore(index),
            FStore { index } => Self::FStore(index),
            DStore { index } => Self::DStore(index),
            AStore { index } => Self::AStore(index),
            IStore0 => Self::IStore0,
            IStore1 => Self::IStore1,
            IStore2 => Self::IStore2,
            IStore3 => Self::IStore3,
            LStore0 => Self::LStore0,
            LStore1 => Self::LStore1,
            LStore2 => Self::LStore2,
            LStore3 => Self::LStore3,
            FStore0 => Self::FStore0,
            FStore1 => Self::FStore1,
            FStore2 => Self::FStore2,
            FStore3 => Self::FStore3,
            DStore0 => Self::DStore0,
            DStore1 => Self::DStore1,
            DStore2 => Self::DStore2,
            DStore3 => Self::DStore3,
            AStore0 => Self::AStore0,
            AStore1 => Self::AStore1,
            AStore2 => Self::AStore2,
            AStore3 => Self::AStore3,
            IAStore => Self::IAStore,
            LAStore => Self::LAStore,
            FAStore => Self::FAStore,
            DAStore => Self::DAStore,
            AAStore => Self::AAStore,
            BAStore => Self::BAStore,
            CAStore => Self::CAStore,
            SAStore => Self::SAStore,

            // Stack
            Pop => Self::Pop,
            Pop2 => Self::Pop2,
            Dup => Self::Dup,
            DupX1 => Self::DupX1,
            DupX2 => Self::DupX2,
            Dup2 => Self::Dup2,
            Dup2X1 => Self::Dup2X1,
            Dup2X2 => Self::Dup2X2,
            Swap => Self::Swap,

            // Math
            IAdd => Self::IAdd,
            LAdd => Self::LAdd,
            FAdd => Self::FAdd,
            DAdd => Self::DAdd,
            ISub => Self::ISub,
            LSub => Self::LSub,
            FSub => Self::FSub,
            DSub => Self::DSub,
            IMul => Self::IMul,
            LMul => Self::LMul,
            FMul => Self::FMul,
            DMul => Self::DMul,
            IDiv => Self::IDiv,
            LDiv => Self::LDiv,
            FDiv => Self::FDiv,
            DDiv => Self::DDiv,
            IRem => Self::IRem,
            LRem => Self::LRem,
            FRem => Self::FRem,
            DRem => Self::DRem,
            INeg => Self::INeg,
            LNeg => Self::LNeg,
            FNeg => Self::FNeg,
            DNeg => Self::DNeg,
            IShl => Self::IShl,
            LShl => Self::LShl,
            IShr => Self::IShr,
            LShr => Self::LShr,
            IUShr => Self::IUShr,
            LUShr => Self::LUShr,
            IAnd => Self::IAnd,
            LAnd => Self::LAnd,
            IOr => Self::IOr,
            LOr => Self::LOr,
            IXor => Self::IXor,
            LXor => Self::LXor,
            IInc { index, constant } => Self::IInc(index, i32::from(constant)),

            // Conversions
            I2L => Self::I2L,
            I2F => Self::I2F,
            I2D => Self::I2D,
            L2I => Self::L2I,
            L2F => Self::L2F,
            L2D => Self::L2D,
            F2I => Self::F2I,
            F2L => Self::F2L,
            F2D => Self::F2D,
            D2I => Self::D2I,
            D2L => Self::D2L,
            D2F => Self::D2F,
            I2B => Self::I2B,
            I2C => Self::I2C,
            I2S => Self::I2S,

            // Comparisons
            LCmp => Self::LCmp,
            FCmpL => Self::FCmpL,
            FCmpG => Self::FCmpG,
            DCmpL => Self::DCmpL,
            DCmpG => Self::DCmpG,
            IfEq { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfEq(target)
            }
            IfNe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfNe(target)
            }
            IfLt { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfLt(target)
            }
            IfGe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfGe(target)
            }
            IfGt { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfGt(target)
            }
            IfLe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfLe(target)
            }
            IfICmpEq { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpEq(target)
            }
            IfICmpNe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpNe(target)
            }
            IfICmpLt { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpLt(target)
            }
            IfICmpGe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpGe(target)
            }
            IfICmpGt { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpGt(target)
            }
            IfICmpLe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfICmpLe(target)
            }
            IfACmpEq { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfACmpEq(target)
            }
            IfACmpNe { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfACmpNe(target)
            }
            Goto { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::Goto(target)
            }
            Jsr { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::Jsr(target)
            }
            Ret { index } => Self::Ret(index),
            TableSwitch {
                default,
                low,
                high,
                jump_offsets,
            } => {
                let targets = jump_offsets
                    .into_iter()
                    .map(|offset| pc.offset(offset))
                    .try_collect()?;
                Self::TableSwitch {
                    default: pc.offset(default)?,
                    range: low..=high,
                    jump_targets: targets,
                }
            }
            LookupSwitch {
                default,
                match_offsets,
            } => {
                let targets = match_offsets
                    .into_iter()
                    .map(|(value, offset)| pc.offset(offset).map(|target| (value, target)))
                    .try_collect()?;
                Self::LookupSwitch {
                    default: pc.offset(default)?,
                    match_targets: targets,
                }
            }
            IReturn => Self::IReturn,
            LReturn => Self::LReturn,
            FReturn => Self::FReturn,
            DReturn => Self::DReturn,
            AReturn => Self::AReturn,
            Return => Self::Return,

            // References
            GetStatic { field_ref_index } => {
                let field_ref = constant_pool.get_field_ref(field_ref_index)?;
                Self::GetStatic(field_ref)
            }
            PutStatic { field_ref_index } => {
                let field_ref = constant_pool.get_field_ref(field_ref_index)?;
                Self::PutStatic(field_ref)
            }
            GetField { field_ref_index } => {
                let field_ref = constant_pool.get_field_ref(field_ref_index)?;
                Self::GetField(field_ref)
            }
            PutField { field_ref_index } => {
                let field_ref = constant_pool.get_field_ref(field_ref_index)?;
                Self::PutField(field_ref)
            }
            InvokeVirtual { method_index } => {
                let method_ref = constant_pool.get_method_ref(method_index)?;
                Self::InvokeVirtual(method_ref)
            }
            InvokeSpecial { method_index } => {
                let method_ref = constant_pool.get_method_ref(method_index)?;
                Self::InvokeSpecial(method_ref)
            }
            InvokeStatic { method_index } => {
                let method_ref = constant_pool.get_method_ref(method_index)?;
                Self::InvokeStatic(method_ref)
            }
            InvokeInterface {
                method_index,
                count,
            } => {
                let method_ref = constant_pool.get_method_ref(method_index)?;
                Self::InvokeInterface(method_ref, count)
            }
            InvokeDynamic { dynamic_index } => {
                let entry = constant_pool.get_entry(dynamic_index)?;
                let &ConstantPoolEntry::InvokeDynamic {
                    bootstrap_method_attr_index: bootstrap_method_index,
                    name_and_type_index,
                } = entry
                else {
                    Err(ClassFileParsingError::MismatchedConstantPoolEntryType {
                        expected: "InvokeDynamic",
                        found: entry.constant_kind(),
                    })?
                };
                let (name, desc_str) = constant_pool.get_name_and_type(name_and_type_index)?;
                let descriptor = MethodDescriptor::from_str(desc_str)?;
                Self::InvokeDynamic {
                    bootstrap_method_index,
                    descriptor,
                    name: name.to_owned(),
                }
            }
            New { index } => {
                let class_ref = constant_pool.get_class_ref(index)?;
                Self::New(class_ref)
            }
            NewArray { atype } => {
                let element_type = match atype {
                    4 => PrimitiveType::Boolean,
                    5 => PrimitiveType::Char,
                    6 => PrimitiveType::Float,
                    7 => PrimitiveType::Double,
                    8 => PrimitiveType::Byte,
                    9 => PrimitiveType::Short,
                    10 => PrimitiveType::Int,
                    11 => PrimitiveType::Long,
                    _ => Err(ClassFileParsingError::MalformedClassFile(
                        "NewArray must create an array of primitive types",
                    ))?,
                };
                Self::NewArray(element_type)
            }
            ANewArray { index } => {
                let element_type = constant_pool.get_class_ref(index)?;
                Self::ANewArray(element_type)
            }
            ArrayLength => Self::ArrayLength,
            AThrow => Self::AThrow,
            CheckCast { target_type_index } => {
                let class_ref = constant_pool.get_type_ref(target_type_index)?;
                Self::CheckCast(class_ref)
            }
            InstanceOf { target_type_index } => {
                let class_ref = constant_pool.get_type_ref(target_type_index)?;
                Self::InstanceOf(class_ref)
            }
            MonitorEnter => Self::MonitorEnter,
            MonitorExit => Self::MonitorExit,

            // Extended
            Wide(raw_wide) => Self::Wide(match raw_wide {
                RawWideInstruction::ILoad { index } => WideInstruction::ILoad(index),
                RawWideInstruction::LLoad { index } => WideInstruction::LLoad(index),
                RawWideInstruction::FLoad { index } => WideInstruction::FLoad(index),
                RawWideInstruction::DLoad { index } => WideInstruction::DLoad(index),
                RawWideInstruction::ALoad { index } => WideInstruction::ALoad(index),
                RawWideInstruction::IStore { index } => WideInstruction::IStore(index),
                RawWideInstruction::LStore { index } => WideInstruction::LStore(index),
                RawWideInstruction::FStore { index } => WideInstruction::FStore(index),
                RawWideInstruction::DStore { index } => WideInstruction::DStore(index),
                RawWideInstruction::AStore { index } => WideInstruction::AStore(index),
                RawWideInstruction::IInc { index, increment } => {
                    WideInstruction::IInc(index, i32::from(increment))
                }
                RawWideInstruction::Ret { index } => WideInstruction::Ret(index),
            }),
            MultiANewArray { index, dimensions } => {
                let class_ref = constant_pool.get_type_ref(index)?;
                Self::MultiANewArray(class_ref, dimensions)
            }
            IfNull { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfNull(target)
            }
            IfNonNull { offset } => {
                let target = pc.offset_i16(offset)?;
                Self::IfNonNull(target)
            }
            GotoW { offset } => {
                let target = pc.offset(offset)?;
                Self::GotoW(target)
            }
            JsrW { offset } => {
                let target = pc.offset(offset)?;
                Self::JsrW(target)
            }

            // Reserved
            Breakpoint => Self::Breakpoint,
            ImpDep1 => Self::ImpDep1,
            ImpDep2 => Self::ImpDep2,
        };

        Ok(result)
    }
}
