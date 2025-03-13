use std::collections::BTreeMap;

use itertools::Itertools;

use crate::{
    jvm::{
        class::{ConstantPool, constant_pool},
        code::{
            Instruction, InstructionList, ProgramCounter, RawInstruction, RawWideInstruction,
            WideInstruction,
        },
        parsing::{Context, Error, ToWriterError, jvm_element_parser::ClassElement},
        references::ClassRef,
    },
    macros::malform,
    types::{Descriptor, field_type::PrimitiveType},
};

impl ClassElement for InstructionList<Instruction> {
    type Raw = InstructionList<RawInstruction>;

    fn from_raw(raw: Self::Raw, ctx: &Context) -> Result<Self, Error> {
        raw.into_iter()
            .map(|(pc, raw_insn)| {
                Instruction::from_raw_instruction(raw_insn, pc, &ctx.constant_pool)
                    .map(|it| (pc, it))
            })
            .try_collect()
    }

    fn into_raw(self, cp: &mut ConstantPool) -> Result<Self::Raw, ToWriterError> {
        let (raw_instructions, _) = self.into_iter().try_fold(
            (BTreeMap::new(), Ok(ProgramCounter::default())),
            |(mut acc, pc), (_, insn)| -> Result<_, ToWriterError> {
                let pc = pc?;
                let raw_insn = insn.into_raw_instruction(pc, cp)?;
                let next_pc = pc + raw_insn.size(pc)?;
                acc.insert(pc, raw_insn);
                Ok((acc, next_pc))
            },
        )?;
        Ok(raw_instructions.into())
    }
}

impl Instruction {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn from_raw_instruction(
        raw_instruction: RawInstruction,
        pc: ProgramCounter,
        constant_pool: &ConstantPool,
    ) -> Result<Self, Error> {
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
                let target = (pc + offset)?;
                Self::IfEq(target)
            }
            IfNe { offset } => {
                let target = (pc + offset)?;
                Self::IfNe(target)
            }
            IfLt { offset } => {
                let target = (pc + offset)?;
                Self::IfLt(target)
            }
            IfGe { offset } => {
                let target = (pc + offset)?;
                Self::IfGe(target)
            }
            IfGt { offset } => {
                let target = (pc + offset)?;
                Self::IfGt(target)
            }
            IfLe { offset } => {
                let target = (pc + offset)?;
                Self::IfLe(target)
            }
            IfICmpEq { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpEq(target)
            }
            IfICmpNe { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpNe(target)
            }
            IfICmpLt { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpLt(target)
            }
            IfICmpGe { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpGe(target)
            }
            IfICmpGt { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpGt(target)
            }
            IfICmpLe { offset } => {
                let target = (pc + offset)?;
                Self::IfICmpLe(target)
            }
            IfACmpEq { offset } => {
                let target = (pc + offset)?;
                Self::IfACmpEq(target)
            }
            IfACmpNe { offset } => {
                let target = (pc + offset)?;
                Self::IfACmpNe(target)
            }
            Goto { offset } => {
                let target = (pc + offset)?;
                Self::Goto(target)
            }
            Jsr { offset } => {
                let target = (pc + offset)?;
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
                    .map(|offset| (pc + offset))
                    .try_collect()?;
                Self::TableSwitch {
                    default: (pc + default)?,
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
                    .map(|(value, offset)| (pc + offset).map(|target| (value, target)))
                    .try_collect()?;
                Self::LookupSwitch {
                    default: (pc + default)?,
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
                let &constant_pool::Entry::InvokeDynamic {
                    bootstrap_method_attr_index: bootstrap_method_index,
                    name_and_type_index,
                } = entry
                else {
                    Err(Error::MismatchedConstantPoolEntryType {
                        expected: "InvokeDynamic",
                        found: entry.constant_kind(),
                    })?
                };
                let (name, descriptor) = constant_pool.get_name_and_type(name_and_type_index)?;
                Self::InvokeDynamic {
                    bootstrap_method_index,
                    name,
                    descriptor,
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
                    _ => malform!("NewArray must create an array of primitive types"),
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
            IfNull { offset } => Self::IfNull((pc + offset)?),
            IfNonNull { offset } => Self::IfNonNull((pc + offset)?),
            GotoW { offset } => Self::GotoW((pc + offset)?),
            JsrW { offset } => Self::JsrW((pc + offset)?),

            // Reserved
            Breakpoint => Self::Breakpoint,
            ImpDep1 => Self::ImpDep1,
            ImpDep2 => Self::ImpDep2,
        };

        Ok(result)
    }

    /// Lower the instruction into a raw instruction.
    ///
    /// # Errors
    /// See [`ToWriterError`] for details.
    #[allow(clippy::too_many_lines)]
    pub fn into_raw_instruction(
        self,
        pc: ProgramCounter,
        cp: &mut ConstantPool,
    ) -> Result<RawInstruction, ToWriterError> {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        let raw = match self {
            // Constants
            Self::Nop => Nop,
            Self::AConstNull => AConstNull,
            Self::IConstM1 => IConstM1,
            Self::IConst0 => IConst0,
            Self::IConst1 => IConst1,
            Self::IConst2 => IConst2,
            Self::IConst3 => IConst3,
            Self::IConst4 => IConst4,
            Self::IConst5 => IConst5,
            Self::LConst0 => LConst0,
            Self::LConst1 => LConst1,
            Self::FConst0 => FConst0,
            Self::FConst1 => FConst1,
            Self::FConst2 => FConst2,
            Self::DConst0 => DConst0,
            Self::DConst1 => DConst1,
            Self::BiPush(value) => BiPush { value },
            Self::SiPush(value) => SiPush { value },
            Self::Ldc(value) | Self::LdcW(value) => {
                let const_index = cp.put_constant_value(value)?;
                if let Ok(const_index) = u8::try_from(const_index) {
                    Ldc { const_index }
                } else {
                    LdcW { const_index }
                }
            }
            Self::Ldc2W(value) => Ldc2W {
                const_index: cp.put_constant_value(value)?,
            },

            // Loads
            Self::ILoad(index) => ILoad { index },
            Self::LLoad(index) => LLoad { index },
            Self::FLoad(index) => FLoad { index },
            Self::DLoad(index) => DLoad { index },
            Self::ALoad(index) => ALoad { index },
            Self::ILoad0 => ILoad0,
            Self::ILoad1 => ILoad1,
            Self::ILoad2 => ILoad2,
            Self::ILoad3 => ILoad3,
            Self::LLoad0 => LLoad0,
            Self::LLoad1 => LLoad1,
            Self::LLoad2 => LLoad2,
            Self::LLoad3 => LLoad3,
            Self::FLoad0 => FLoad0,
            Self::FLoad1 => FLoad1,
            Self::FLoad2 => FLoad2,
            Self::FLoad3 => FLoad3,
            Self::DLoad0 => DLoad0,
            Self::DLoad1 => DLoad1,
            Self::DLoad2 => DLoad2,
            Self::DLoad3 => DLoad3,
            Self::ALoad0 => ALoad0,
            Self::ALoad1 => ALoad1,
            Self::ALoad2 => ALoad2,
            Self::ALoad3 => ALoad3,
            Self::IALoad => IALoad,
            Self::LALoad => LALoad,
            Self::FALoad => FALoad,
            Self::DALoad => DALoad,
            Self::AALoad => AALoad,
            Self::BALoad => BALoad,
            Self::CALoad => CALoad,
            Self::SALoad => SALoad,

            // Stores
            Self::IStore(index) => IStore { index },
            Self::LStore(index) => LStore { index },
            Self::FStore(index) => FStore { index },
            Self::DStore(index) => DStore { index },
            Self::AStore(index) => AStore { index },
            Self::IStore0 => IStore0,
            Self::IStore1 => IStore1,
            Self::IStore2 => IStore2,
            Self::IStore3 => IStore3,
            Self::LStore0 => LStore0,
            Self::LStore1 => LStore1,
            Self::LStore2 => LStore2,
            Self::LStore3 => LStore3,
            Self::FStore0 => FStore0,
            Self::FStore1 => FStore1,
            Self::FStore2 => FStore2,
            Self::FStore3 => FStore3,
            Self::DStore0 => DStore0,
            Self::DStore1 => DStore1,
            Self::DStore2 => DStore2,
            Self::DStore3 => DStore3,
            Self::AStore0 => AStore0,
            Self::AStore1 => AStore1,
            Self::AStore2 => AStore2,
            Self::AStore3 => AStore3,
            Self::IAStore => IAStore,
            Self::LAStore => LAStore,
            Self::FAStore => FAStore,
            Self::DAStore => DAStore,
            Self::AAStore => AAStore,
            Self::BAStore => BAStore,
            Self::CAStore => CAStore,
            Self::SAStore => SAStore,

            // Stack
            Self::Pop => Pop,
            Self::Pop2 => Pop2,
            Self::Dup => Dup,
            Self::DupX1 => DupX1,
            Self::DupX2 => DupX2,
            Self::Dup2 => Dup2,
            Self::Dup2X1 => Dup2X1,
            Self::Dup2X2 => Dup2X2,
            Self::Swap => Swap,

            // Math
            Self::IAdd => IAdd,
            Self::LAdd => LAdd,
            Self::FAdd => FAdd,
            Self::DAdd => DAdd,
            Self::ISub => ISub,
            Self::LSub => LSub,
            Self::FSub => FSub,
            Self::DSub => DSub,
            Self::IMul => IMul,
            Self::LMul => LMul,
            Self::FMul => FMul,
            Self::DMul => DMul,
            Self::IDiv => IDiv,
            Self::LDiv => LDiv,
            Self::FDiv => FDiv,
            Self::DDiv => DDiv,
            Self::IRem => IRem,
            Self::LRem => LRem,
            Self::FRem => FRem,
            Self::DRem => DRem,
            Self::INeg => INeg,
            Self::LNeg => LNeg,
            Self::FNeg => FNeg,
            Self::DNeg => DNeg,
            Self::IShl => IShl,
            Self::LShl => LShl,
            Self::IShr => IShr,
            Self::LShr => LShr,
            Self::IUShr => IUShr,
            Self::LUShr => LUShr,
            Self::IAnd => IAnd,
            Self::LAnd => LAnd,
            Self::IOr => IOr,
            Self::LOr => LOr,
            Self::IXor => IXor,
            Self::LXor => LXor,
            Self::IInc(index, increment) => IInc {
                index,
                constant: i8::try_from(increment)?,
            },

            // Conversions
            Self::I2L => I2L,
            Self::I2F => I2F,
            Self::I2D => I2D,
            Self::L2I => L2I,
            Self::L2F => L2F,
            Self::L2D => L2D,
            Self::F2I => F2I,
            Self::F2L => F2L,
            Self::F2D => F2D,
            Self::D2I => D2I,
            Self::D2L => D2L,
            Self::D2F => D2F,
            Self::I2B => I2B,
            Self::I2C => I2C,
            Self::I2S => I2S,

            // Comparisons
            Self::LCmp => LCmp,
            Self::FCmpL => FCmpL,
            Self::FCmpG => FCmpG,
            Self::DCmpL => DCmpL,
            Self::DCmpG => DCmpG,
            Self::IfEq(target) => IfEq {
                offset: try_offset(target, pc)?,
            },
            Self::IfNe(target) => IfNe {
                offset: try_offset(target, pc)?,
            },
            Self::IfLt(target) => IfLt {
                offset: try_offset(target, pc)?,
            },
            Self::IfGe(target) => IfGe {
                offset: try_offset(target, pc)?,
            },
            Self::IfGt(target) => IfGt {
                offset: try_offset(target, pc)?,
            },
            Self::IfLe(target) => IfLe {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpEq(target) => IfICmpEq {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpNe(target) => IfICmpNe {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpLt(target) => IfICmpLt {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpGe(target) => IfICmpGe {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpGt(target) => IfICmpGt {
                offset: try_offset(target, pc)?,
            },
            Self::IfICmpLe(target) => IfICmpLe {
                offset: try_offset(target, pc)?,
            },
            Self::IfACmpEq(target) => IfACmpEq {
                offset: try_offset(target, pc)?,
            },
            Self::IfACmpNe(target) => IfACmpNe {
                offset: try_offset(target, pc)?,
            },
            Self::Goto(target) => Goto {
                offset: try_offset(target, pc)?,
            },
            Self::Jsr(target) => Jsr {
                offset: try_offset(target, pc)?,
            },
            Self::Ret(index) => Ret { index },

            Self::TableSwitch {
                default,
                range,
                jump_targets,
            } => {
                let low = *range.start();
                let high = *range.end();
                let jump_offsets = jump_targets
                    .into_iter()
                    .map(|target| offset_wide(target, pc))
                    .collect();
                TableSwitch {
                    default: (u16::from(default) - u16::from(pc)).into(),
                    low,
                    high,
                    jump_offsets,
                }
            }
            Self::LookupSwitch {
                default,
                match_targets,
            } => {
                let match_offsets = match_targets
                    .into_iter()
                    .map(|(value, target)| (value, offset_wide(target, pc)))
                    .collect();
                LookupSwitch {
                    default: offset_wide(default, pc),
                    match_offsets,
                }
            }

            // Return
            Self::IReturn => IReturn,
            Self::LReturn => LReturn,
            Self::FReturn => FReturn,
            Self::DReturn => DReturn,
            Self::AReturn => AReturn,
            Self::Return => Return,

            // References
            Self::GetStatic(field_ref) => GetStatic {
                field_ref_index: cp.put_field_ref(field_ref)?,
            },
            Self::PutStatic(field_ref) => PutStatic {
                field_ref_index: cp.put_field_ref(field_ref)?,
            },
            Self::GetField(field_ref) => GetField {
                field_ref_index: cp.put_field_ref(field_ref)?,
            },
            Self::PutField(field_ref) => PutField {
                field_ref_index: cp.put_field_ref(field_ref)?,
            },
            Self::InvokeVirtual(method_ref) => InvokeVirtual {
                method_index: cp.put_method_ref(method_ref)?,
            },
            Self::InvokeSpecial(method_ref) => InvokeSpecial {
                method_index: cp.put_method_ref(method_ref)?,
            },
            Self::InvokeStatic(method_ref) => InvokeStatic {
                method_index: cp.put_method_ref(method_ref)?,
            },
            Self::InvokeInterface(method_ref, count) => InvokeInterface {
                method_index: cp.put_method_ref(method_ref)?,
                count,
            },
            Self::InvokeDynamic {
                bootstrap_method_index,
                name,
                ref descriptor,
            } => {
                let name_and_type_index = cp.put_name_and_type(name, descriptor)?;
                let entry = constant_pool::Entry::InvokeDynamic {
                    bootstrap_method_attr_index: bootstrap_method_index,
                    name_and_type_index,
                };
                let dynamic_index = cp.put_entry(entry)?;
                InvokeDynamic { dynamic_index }
            }
            Self::New(class_ref) => New {
                index: cp.put_class_ref(class_ref)?,
            },
            Self::NewArray(atype) => NewArray {
                atype: match atype {
                    PrimitiveType::Boolean => 4,
                    PrimitiveType::Char => 5,
                    PrimitiveType::Float => 6,
                    PrimitiveType::Double => 7,
                    PrimitiveType::Byte => 8,
                    PrimitiveType::Short => 9,
                    PrimitiveType::Int => 10,
                    PrimitiveType::Long => 11,
                },
            },
            Self::ANewArray(class_ref) => ANewArray {
                index: cp.put_class_ref(class_ref)?,
            },
            Self::ArrayLength => ArrayLength,
            Self::AThrow => AThrow,
            Self::CheckCast(type_ref) => CheckCast {
                target_type_index: cp.put_type_ref(type_ref)?,
            },
            Self::InstanceOf(type_ref) => InstanceOf {
                target_type_index: cp.put_type_ref(type_ref)?,
            },
            Self::MonitorEnter => MonitorEnter,
            Self::MonitorExit => MonitorExit,

            // Extended
            Self::Wide(raw_wide) => Wide(match raw_wide {
                WideInstruction::ILoad(index) => RawWideInstruction::ILoad { index },
                WideInstruction::LLoad(index) => RawWideInstruction::LLoad { index },
                WideInstruction::FLoad(index) => RawWideInstruction::FLoad { index },
                WideInstruction::DLoad(index) => RawWideInstruction::DLoad { index },
                WideInstruction::ALoad(index) => RawWideInstruction::ALoad { index },
                WideInstruction::IStore(index) => RawWideInstruction::IStore { index },
                WideInstruction::LStore(index) => RawWideInstruction::LStore { index },
                WideInstruction::FStore(index) => RawWideInstruction::FStore { index },
                WideInstruction::DStore(index) => RawWideInstruction::DStore { index },
                WideInstruction::AStore(index) => RawWideInstruction::AStore { index },
                WideInstruction::IInc(index, increment) => RawWideInstruction::IInc {
                    index,
                    increment: increment.try_into()?,
                },
                WideInstruction::Ret(index) => RawWideInstruction::Ret { index },
            }),
            Self::MultiANewArray(element_type, dimensions) => {
                let index = cp.put_class_ref(ClassRef {
                    binary_name: element_type.descriptor(),
                })?;
                MultiANewArray { index, dimensions }
            }
            Self::IfNull(target) => IfNull {
                offset: try_offset(target, pc)?,
            },
            Self::IfNonNull(target) => IfNonNull {
                offset: try_offset(target, pc)?,
            },
            Self::GotoW(target) => GotoW {
                offset: offset_wide(target, pc),
            },
            Self::JsrW(target) => JsrW {
                offset: offset_wide(target, pc),
            },

            // Reserved
            Self::Breakpoint => Breakpoint,
            Self::ImpDep1 => ImpDep1,
            Self::ImpDep2 => ImpDep2,
        };

        Ok(raw)
    }
}

fn try_offset(target: ProgramCounter, pc: ProgramCounter) -> Result<i16, ToWriterError> {
    let target: i32 = target.into();
    let pc: i32 = pc.into();
    let offset = target - pc;
    let offset = i16::try_from(offset)?;
    Ok(offset)
}

fn offset_wide(target: ProgramCounter, pc: ProgramCounter) -> i32 {
    let target: i32 = target.into();
    let pc: i32 = pc.into();
    target - pc
}
