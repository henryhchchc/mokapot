use std::io::{self, Write};

use super::{RawInstruction, RawWideInstruction};
use crate::jvm::bytecode::{GenerationError, reader::PositionTracker, write_length};

impl RawInstruction {
    /// Writes a single raw instruction to the given writer.
    #[allow(clippy::too_many_lines)]
    pub(super) fn write_one<W>(
        &self,
        writer: &mut PositionTracker<W>,
    ) -> Result<(), GenerationError>
    where
        PositionTracker<W>: io::Write,
    {
        #[allow(clippy::enum_glob_use)]
        use RawInstruction::*;

        writer.write_all(&[self.opcode()])?;

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
            | AStore { index }
            | Ret { index } => writer.write_all(&index.to_be_bytes())?,
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
            | Jsr { offset }
            | IfNull { offset }
            | IfNonNull { offset } => writer.write_all(&offset.to_be_bytes())?,
            TableSwitch {
                default,
                low,
                high,
                jump_offsets,
            } => {
                while !writer.position().is_multiple_of(4) {
                    writer.write_all(&[0])?;
                }
                writer.write_all(&default.to_be_bytes())?;
                writer.write_all(&low.to_be_bytes())?;
                writer.write_all(&high.to_be_bytes())?;
                for offset in jump_offsets {
                    writer.write_all(&offset.to_be_bytes())?;
                }
            }
            LookupSwitch {
                default,
                match_offsets,
            } => {
                while !writer.position().is_multiple_of(4) {
                    writer.write_all(&[0])?;
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
                writer.write_all(&[0])?;
            }
            InvokeDynamic { dynamic_index } => {
                writer.write_all(&dynamic_index.to_be_bytes())?;
                writer.write_all(&[0, 0])?;
            }
            New { index } | ANewArray { index } => writer.write_all(&index.to_be_bytes())?,
            NewArray { atype } => writer.write_all(&atype.to_be_bytes())?,
            CheckCast { target_type_index } | InstanceOf { target_type_index } => {
                writer.write_all(&target_type_index.to_be_bytes())?;
            }
            Wide(instruction) => {
                writer.write_all(&[instruction.opcode()])?;
                match instruction {
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
            GotoW { offset } | JsrW { offset } => writer.write_all(&offset.to_be_bytes())?,
            _ => {}
        }
        Ok(())
    }
}
