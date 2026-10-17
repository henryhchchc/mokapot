pub(crate) mod instruction_conversion;
pub(crate) mod local_variable;
pub(crate) mod raw_instruction;
pub(super) mod stack_map;

use std::result::Result;

pub(crate) use local_variable::{LocalVariableDescAttr, LocalVariableTypeAttr};

use super::{
    ParseError, ParsingContext,
    class_element::ClassElement,
    raw_attributes::{self, Code},
};
use crate::{
    intrinsics::extract_attributes,
    jvm::{
        Annotations,
        bytecode::code::raw_instruction::RawInstruction,
        code::{
            ExceptionTableEntry, Instruction, LineNumberTableEntry, LocalVariableTable, MethodBody,
            ProgramCounter,
        },
        method::{ParameterAccessFlags, ParameterInfo},
    },
};

impl ClassElement<'_> for LineNumberTableEntry {
    type Raw = raw_attributes::LineNumberTableEntry;

    fn from_raw(raw: Self::Raw, _ctx: &ParsingContext) -> Result<Self, ParseError> {
        Ok(Self {
            start_pc: raw.start_pc.into(),
            line_number: raw.line_number.get(),
        })
    }
}

impl ClassElement<'_> for ExceptionTableEntry {
    type Raw = raw_attributes::ExceptionTableEntry;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let raw_attributes::ExceptionTableEntry {
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

impl ClassElement<'_> for ParameterInfo {
    type Raw = raw_attributes::ParameterInfo;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
        let raw_attributes::ParameterInfo {
            name_index,
            access_flags,
        } = raw;
        let name_index = name_index.get();
        let access_flags = access_flags.get();
        let name = if name_index == 0 {
            None
        } else {
            Some(ctx.constant_pool.get_str(name_index)?.to_owned())
        };
        let access_flags = ParameterAccessFlags::from_bits(access_flags)
            .ok_or(ParseError::malform("Invalid parameter access flags"))?;
        Ok(ParameterInfo { name, access_flags })
    }
}

impl<'a> ClassElement<'a> for MethodBody {
    type Raw = Code<'a>;

    fn from_raw(raw: Self::Raw, ctx: &ParsingContext) -> Result<Self, ParseError> {
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
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
        let attributes: Vec<Attribute> = attributes
            .into_iter()
            .map(|it| ClassElement::from_raw(it, ctx))
            .collect::<Result<_, _>>()?;
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
                    for LocalVariableDescAttr { id, name, field_type } in it {
                        table.merge_type(id, name, field_type)?;
                    }
                },
                match Attribute::LocalVariableTypeTable(it) => {
                    let table = local_variable_table.get_or_insert(LocalVariableTable::default());
                    for LocalVariableTypeAttr { id, name, signature } in it {
                        table.merge_signature(id, name, signature)?;
                    }
                },
                else let other_attributes
            }
        }

        Ok(Self {
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
    use crate::jvm::class::ConstantPool;
    use crate::jvm::{class::Version, errors::ParseErrorKind};

    fn context() -> ParsingContext {
        ParsingContext {
            constant_pool: ConstantPool::new(),
            class_version: Version::LATEST,
            current_class_name: "Test".parse().unwrap(),
        }
    }

    #[test]
    fn exception_table_entry_preserves_exclusive_end_pc() {
        let context = context();
        let entry = ExceptionTableEntry::from_raw(
            raw_attributes::ExceptionTableEntry {
                start_pc: 1.into(),
                end_pc: 4.into(),
                handler_pc: 5.into(),
                catch_type: 0.into(),
            },
            &context,
        )
        .unwrap();

        assert_eq!(
            entry.covered_pc,
            ProgramCounter::from(1)..ProgramCounter::from(4)
        );
    }

    #[test]
    fn method_body_propagates_malformed_instruction_bytes() {
        let context = context();
        let error = MethodBody::from_raw(
            raw_attributes::Code {
                max_stack: 0,
                max_locals: 0,
                code: &[0xcb],
                exception_table: &[],
                attributes: Box::new([]),
            },
            &context,
        )
        .unwrap_err();
        assert_eq!(error.kind(), ParseErrorKind::Malformed);
    }
}
