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
    Annotations,
    class_file::code::raw_instruction::RawInstruction,
    code::{
        ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableTable, MethodBody,
        ProgramCounter,
    },
};

impl From<raw::LineNumberTableEntry> for LineNumberTableEntry {
    fn from(raw: raw::LineNumberTableEntry) -> Self {
        Self {
            start_pc: raw.start_pc.into(),
            line_number: raw.line_number.get(),
        }
    }
}

impl Resolve for raw::ExceptionTableEntry {
    type Output = ExceptionTableEntry;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
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
            Some(ctx.constant_pool.get_class_name(catch_type)?)
        };
        Ok(ExceptionTableEntry {
            covered_pc,
            handler_pc,
            catch_type,
        })
    }
}

impl Resolve for Code<'_> {
    type Output = MethodBody;

    fn resolve(self, ctx: &ResolutionContext) -> Result<Self::Output, ParseError> {
        let raw = self;
        let Code {
            max_stack,
            max_locals,
            code,
            exception_table,
            attributes,
        } = raw;

        let instructions = RawInstruction::iter_from_bytes(code)
            .map(|entry| {
                let (pc, raw) = entry?;
                Instruction::from_raw_instruction(raw, pc, &ctx.constant_pool).map(|it| (pc, it))
            })
            .collect::<Result<_, ParseError>>()?;

        let exception_table = exception_table
            .iter()
            .copied()
            .map(|it| it.resolve(ctx))
            .try_collect_exact()?;
        let attributes = attributes.into_iter().map(|it| it.resolve(ctx));
        let mut local_variable_table = None;
        extract_attributes! {
            for attributes in "code" {
                let line_number_table: LineNumberTable,
                let stack_map_table: StackMapTable,
                let runtime_visible_type_annotations:
                    RuntimeVisibleTypeAnnotations as unwrap_or_default,
                let runtime_invisible_type_annotations:
                    RuntimeInvisibleTypeAnnotations as unwrap_or_default,
                match Attribute::LocalVariableTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::default());
                    for LocalVariableDescriptorEntry { id, name, field_type } in it {
                        table.merge_type(id, name, field_type)?;
                    }
                },
                match Attribute::LocalVariableTypeTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::default());
                    for LocalVariableSignatureEntry { id, name, signature } in it {
                        table.merge_signature(id, name, signature)?;
                    }
                },
                else let other_attributes
            }
        }

        Ok(MethodBody {
            max_stack,
            max_locals,
            instructions,
            exception_table,
            line_number_table,
            local_variable_table,
            stack_map_table,
            type_annotations: Annotations {
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
    use crate::jvm::class_file::constant_pool::ConstantPool;

    fn context() -> ResolutionContext<'static> {
        ResolutionContext {
            constant_pool: ConstantPool::new(),
            class_version: Version::LATEST,
            current_class_name: "Test".parse().unwrap(),
        }
    }

    #[test]
    fn exception_table_entry_preserves_exclusive_end_pc() {
        let context = context();
        let entry = raw::ExceptionTableEntry {
            start_pc: 1.into(),
            end_pc: 4.into(),
            handler_pc: 5.into(),
            catch_type: 0.into(),
        }
        .resolve(&context)
        .unwrap();

        assert_eq!(
            entry.covered_pc,
            ProgramCounter::from(1)..ProgramCounter::from(4)
        );
    }
}
