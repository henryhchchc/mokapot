use crate::intrinsics::ExactSizeIteratorExt;

use crate::jvm::class_file::attribute::extract_attributes;

pub(crate) mod local_variable;
pub(super) mod raw;
pub(crate) mod raw_instruction;
mod resolve;
mod stack_map;

pub(crate) use local_variable::{LocalVariableDescriptorEntry, LocalVariableSignatureEntry};

use super::{ParseError, ResolutionContext, code::raw::Code, resolve::Resolve};
use crate::jvm::{
    annotation::AnnotationsData,
    class_file::code::raw_instruction::RawInstruction,
    code::{
        ExceptionTableEntryData, InstructionData, InstructionStorage, LineNumberTableEntry,
        LocalVariableTableData, MethodBodyData, ProgramCounter,
    },
};
use zerocopy::byteorder::big_endian::U16;

impl From<U16> for ProgramCounter {
    fn from(value: U16) -> Self {
        Self::from(value.get())
    }
}

impl From<raw::LineNumberTableEntry> for LineNumberTableEntry {
    fn from(raw: raw::LineNumberTableEntry) -> Self {
        Self {
            start_pc: raw.start_pc.into(),
            line_number: raw.line_number.get(),
        }
    }
}

impl Resolve for raw::ExceptionTableEntry {
    type Output = ExceptionTableEntryData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let raw::ExceptionTableEntry {
            start_pc,
            end_pc,
            handler_pc,
            catch_type,
        } = raw;
        let catch_type = catch_type.get();
        let start_pc = ProgramCounter::from(start_pc);
        let end_pc = ProgramCounter::from(end_pc);
        let covered_pc = start_pc..end_pc;
        let handler_pc = ProgramCounter::from(handler_pc);
        let catch_type = if catch_type == 0 {
            None
        } else {
            Some(ctx.constant_pool.class_id(catch_type)?)
        };
        Ok(ExceptionTableEntryData {
            covered_pc,
            handler_pc,
            catch_type,
        })
    }
}

impl Resolve for Code<'_> {
    type Output = MethodBodyData;

    fn resolve(self, ctx: &mut ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let Code {
            max_stack,
            max_locals,
            code,
            exception_table,
            attributes,
        } = raw;

        let mut instructions = Vec::with_capacity(code.len().min(usize::from(u16::MAX)));
        for entry in RawInstruction::iter_from_bytes(code) {
            let (pc, raw) = entry?;
            let instruction =
                InstructionData::from_raw_instruction(raw, pc, &mut ctx.constant_pool)?;
            instructions.push((pc, instruction));
        }
        instructions.shrink_to_fit();
        let instructions = InstructionStorage::from_ordered(instructions);

        let exception_table = exception_table
            .iter()
            .copied()
            .map(|it| it.resolve(ctx))
            .try_collect_exact()?;
        let mut line_number_table: Option<Vec<LineNumberTableEntry>> = None;
        let mut local_variable_table = None;
        extract_attributes! {
            for attributes in "code" resolving ctx {
                let stack_map_table: StackMapTable,
                let runtime_visible_type_annotations:
                    RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations:
                    RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                match Attribute::LineNumberTable(it) => {
                    line_number_table.get_or_insert_with(Vec::new).extend(it);
                },
                match Attribute::LocalVariableTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTableData::default());
                    for LocalVariableDescriptorEntry { id, name, field_type } in it {
                        local_variable::merge_type(table, id, name, field_type, &ctx.constant_pool)?;
                    }
                },
                match Attribute::LocalVariableTypeTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTableData::default());
                    for LocalVariableSignatureEntry { id, name, signature } in it {
                        local_variable::merge_signature(table, id, name, signature, &ctx.constant_pool)?;
                    }
                },
                else let other_attributes
            }
        }

        Ok(MethodBodyData {
            max_stack,
            max_locals,
            instructions,
            exception_table,
            line_number_table: line_number_table.map(Vec::into_boxed_slice),
            local_variable_table,
            stack_map_table,
            type_annotations: AnnotationsData {
                runtime_visible: runtime_visible_type_annotations,
                runtime_invisible: runtime_invisible_type_annotations,
            },
            other_attributes,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::jvm::class::Version;
    use crate::jvm::class_file::constant_pool::{RawConstantPool, resolve};

    fn context() -> ResolutionContext {
        ResolutionContext {
            constant_pool: resolve(RawConstantPool::from_entries([])).unwrap(),
            class_version: Version::LATEST,
        }
    }

    #[test]
    fn line_number_tables_are_merged_in_attribute_order() {
        use crate::jvm::class_file::{Decode, attribute::AttributeInfo, constant_pool::RawEntry};

        let pool = RawConstantPool::from_entries([RawEntry::Utf8(b"LineNumberTable")]);
        let mut context = ResolutionContext {
            constant_pool: resolve(pool).unwrap(),
            class_version: Version::LATEST,
        };
        let first = [0, 1, 0, 0, 0, 6, 0, 1, 0, 0, 0, 10];
        let second = [0, 1, 0, 0, 0, 6, 0, 1, 0, 1, 0, 20];
        let empty = [0, 1, 0, 0, 0, 2, 0, 0];
        let first = <&AttributeInfo>::decode(&mut first.as_slice()).unwrap();
        let second = <&AttributeInfo>::decode(&mut second.as_slice()).unwrap();
        let empty = <&AttributeInfo>::decode(&mut empty.as_slice()).unwrap();
        let mut resolve = |attributes: Box<[&AttributeInfo]>| {
            Code {
                max_stack: 0,
                max_locals: 0,
                code: &[0, 0],
                exception_table: &[],
                attributes,
            }
            .resolve(&mut context)
            .unwrap()
            .line_number_table
        };
        assert!(resolve(Box::new([])).is_none());
        assert!(resolve(Box::new([empty])).unwrap().is_empty());
        let entries = resolve(Box::new([first, second])).unwrap();
        let entries: Vec<_> = entries
            .iter()
            .map(|it| (it.start_pc, it.line_number))
            .collect();
        assert_eq!(entries, [(0.into(), 10), (1.into(), 20)]);
    }

    #[test]
    fn instructions_preserve_offsets_and_reject_invalid_bytecode() {
        use crate::jvm::errors::ParseErrorKind;

        let mut context = context();
        let mut resolve = |code| {
            Code {
                max_stack: 2,
                max_locals: 0,
                code,
                exception_table: &[],
                attributes: Box::new([]),
            }
            .resolve(&mut context)
        };
        let body = resolve(&[0x10, 42, 0x04, 0x60, 0xac]).unwrap();
        let instructions: Vec<_> = body.instructions.into_iter().collect();
        assert_eq!(
            instructions,
            [
                (0.into(), InstructionData::BiPush(42)),
                (2.into(), InstructionData::IConst1),
                (3.into(), InstructionData::IAdd),
                (4.into(), InstructionData::IReturn),
            ]
        );
        for (bytes, expected) in [
            (
                &[0x10][..],
                ParseErrorKind::Truncated {
                    requested: 1,
                    available: 0,
                },
            ),
            (
                &[0xb2, 0, 1][..],
                ParseErrorKind::InvalidConstantPoolIndex { index: 1 },
            ),
            (
                &[0xcb][..],
                ParseErrorKind::InvalidOpcode {
                    opcode: 0xcb,
                    pc: 0,
                },
            ),
        ] {
            assert_eq!(resolve(bytes).unwrap_err().kind(), expected);
        }
    }

    #[test]
    fn catch_types_reject_array_and_descriptor_class_entries() {
        use crate::jvm::{
            class_file::constant_pool::{RawConstantPool, RawEntry},
            errors::ParseErrorKind,
        };

        for name in [b"[I".as_slice(), b"Ljava/lang/Throwable;"] {
            let pool = RawConstantPool::from_entries([
                RawEntry::Utf8(name),
                RawEntry::Class {
                    name_index: 1.into(),
                },
            ]);
            let mut ctx = ResolutionContext {
                constant_pool: resolve(pool).unwrap(),
                class_version: Version::LATEST,
            };
            let entry = raw::ExceptionTableEntry {
                start_pc: 0.into(),
                end_pc: 1.into(),
                handler_pc: 1.into(),
                catch_type: 2.into(),
            };
            assert_eq!(
                entry.resolve(&mut ctx).unwrap_err().kind(),
                ParseErrorKind::UnexpectedConstantPoolEntry {
                    index: 2,
                    expected: "ClassName",
                    actual: "CONSTANT_Class"
                }
            );
        }
    }

    #[test]
    fn exception_table_entry_preserves_exclusive_end_pc() {
        let mut context = context();
        let entry = raw::ExceptionTableEntry {
            start_pc: 1.into(),
            end_pc: 4.into(),
            handler_pc: 5.into(),
            catch_type: 0.into(),
        }
        .resolve(&mut context)
        .unwrap();

        assert_eq!(
            entry.covered_pc,
            ProgramCounter::from(1)..ProgramCounter::from(4)
        );
    }
}
