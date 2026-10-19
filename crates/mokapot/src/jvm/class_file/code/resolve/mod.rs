//! Resolves raw JVM instruction models.

use crate::intrinsics::ExactSizeIteratorExt;

use itertools::Itertools;

use super::raw_instruction::{RawInstruction, RawWideInstruction};
use crate::{
    jvm::{
        class_file::constant_pool::ConstantPool,
        code::{InstructionData, ProgramCounter, WideInstruction},
        errors::ParseError,
        errors::ParsingErrorContext,
    },
    types::field_type::PrimitiveType,
};

impl InstructionData {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn from_raw_instruction(
        raw_instruction: RawInstruction<'_>,
        pc: ProgramCounter,
        constant_pool: &mut ConstantPool,
    ) -> Result<Self, ParseError> {
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
                let constant = constant_pool.constant_value_id(u16::from(const_index))?;
                Self::Ldc(constant)
            }
            LdcW { const_index } => {
                let constant = constant_pool.constant_value_id(const_index)?;
                Self::LdcW(constant)
            }
            Ldc2W { const_index } => {
                let constant = constant_pool.constant_value_id(const_index)?;
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
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfEq(target)
            }
            IfNe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfNe(target)
            }
            IfLt { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfLt(target)
            }
            IfGe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfGe(target)
            }
            IfGt { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfGt(target)
            }
            IfLe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfLe(target)
            }
            IfICmpEq { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpEq(target)
            }
            IfICmpNe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpNe(target)
            }
            IfICmpLt { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpLt(target)
            }
            IfICmpGe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpGe(target)
            }
            IfICmpGt { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpGt(target)
            }
            IfICmpLe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfICmpLe(target)
            }
            IfACmpEq { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfACmpEq(target)
            }
            IfACmpNe { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::IfACmpNe(target)
            }
            Goto { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::Goto(target)
            }
            Jsr { offset } => {
                let target = (pc + offset).context("Invalid jump offset")?;
                Self::Jsr(target)
            }
            Ret { index } => Self::Ret(index),
            TableSwitch(info) => {
                let jump_targets = info
                    .jump_offsets
                    .iter()
                    .map(|offset| (pc + offset.get()).context("Invalid jump offset"))
                    .try_collect_exact()?;
                Self::TableSwitch {
                    low: info.low.get(),
                    jump_targets,
                    default: (pc + info.default.get()).context("Invalid jump offset")?,
                }
            }
            LookupSwitch(info) => {
                let targets = info
                    .match_offsets
                    .iter()
                    .map(|pair| {
                        let value = pair.match_value.get();
                        let offset = pair.offset.get();
                        (pc + offset)
                            .map(|target| (value, target))
                            .context("Invalid jump offset")
                    })
                    .try_collect()?;
                Self::LookupSwitch {
                    default: (pc + info.default.get()).context("Invalid jump offset")?,
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
                let field_ref = constant_pool.field_ref_id(field_ref_index)?;
                Self::GetStatic(field_ref)
            }
            PutStatic { field_ref_index } => {
                let field_ref = constant_pool.field_ref_id(field_ref_index)?;
                Self::PutStatic(field_ref)
            }
            GetField { field_ref_index } => {
                let field_ref = constant_pool.field_ref_id(field_ref_index)?;
                Self::GetField(field_ref)
            }
            PutField { field_ref_index } => {
                let field_ref = constant_pool.field_ref_id(field_ref_index)?;
                Self::PutField(field_ref)
            }
            InvokeVirtual { method_index } => {
                let method_ref = constant_pool.method_ref_id(method_index)?;
                Self::InvokeVirtual(method_ref)
            }
            InvokeSpecial { method_index } => {
                let method_ref = constant_pool.method_or_interface_ref_id(method_index)?;
                Self::InvokeSpecial(method_ref)
            }
            InvokeStatic { method_index } => {
                let method_ref = constant_pool.method_or_interface_ref_id(method_index)?;
                Self::InvokeStatic(method_ref)
            }
            InvokeInterface {
                method_index,
                count,
            } => {
                let method_ref = constant_pool.interface_method_ref_id(method_index)?;
                Self::InvokeInterface(method_ref, count)
            }
            InvokeDynamic { dynamic_index } => {
                Self::InvokeDynamic(constant_pool.invoke_dynamic_id(dynamic_index)?)
            }
            New { index } => {
                let class_name = constant_pool.class_id(index)?;
                Self::New(class_name)
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
                    _ => Err(ParseError::malform(
                        "NewArray must create an array of primitive types",
                    ))?,
                };
                Self::NewArray(element_type)
            }
            ANewArray { index } => {
                let element_type = constant_pool.reference_type_id(index)?;
                Self::ANewArray(element_type)
            }
            ArrayLength => Self::ArrayLength,
            AThrow => Self::AThrow,
            CheckCast { target_type_index } => {
                let type_ref = constant_pool.reference_type_id(target_type_index)?;
                Self::CheckCast(type_ref)
            }
            InstanceOf { target_type_index } => {
                let type_ref = constant_pool.reference_type_id(target_type_index)?;
                Self::InstanceOf(type_ref)
            }
            MonitorEnter => Self::MonitorEnter,
            MonitorExit => Self::MonitorExit,

            // Extended
            Wide(raw_wide) => Self::Wide(lift_wide_instruction(raw_wide)),
            MultiANewArray { index, dimensions } => {
                let type_ref = constant_pool.reference_type_id(index)?;
                Self::MultiANewArray(type_ref, dimensions)
            }
            IfNull { offset } => Self::IfNull((pc + offset).context("Invalid jump offset")?),
            IfNonNull { offset } => Self::IfNonNull((pc + offset).context("Invalid jump offset")?),
            GotoW { offset } => Self::GotoW((pc + offset).context("Invalid jump offset")?),
            JsrW { offset } => Self::JsrW((pc + offset).context("Invalid jump offset")?),

            // Reserved
            Breakpoint => Self::Breakpoint,
            ImpDep1 => Self::ImpDep1,
            ImpDep2 => Self::ImpDep2,
        };

        Ok(result)
    }
}

fn lift_wide_instruction(raw_wide: RawWideInstruction) -> WideInstruction {
    match raw_wide {
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
    }
}

#[cfg(test)]
mod tests;
