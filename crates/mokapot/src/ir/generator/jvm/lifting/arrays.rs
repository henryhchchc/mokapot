use crate::{
    ir::{
        expression::{ArrayOperation, Expression},
        generator::{
            error::MokaIRBuildError,
            identity::SsaValueId,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                lifting::require_definition_id,
                symbolic_execution::Value,
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::field_type::FieldType,
};

pub(super) const fn produces_value(instruction: &JVM) -> bool {
    matches!(
        instruction,
        JVM::IALoad
            | JVM::FALoad
            | JVM::AALoad
            | JVM::BALoad
            | JVM::CALoad
            | JVM::SALoad
            | JVM::LALoad
            | JVM::DALoad
            | JVM::ANewArray(_)
            | JVM::NewArray(_)
            | JVM::MultiANewArray(_, _)
            | JVM::ArrayLength
    )
}

#[expect(
    clippy::too_many_lines,
    reason = "the match is an exhaustive opcode-family dispatch"
)]
pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    #[allow(
        clippy::enum_glob_use,
        reason = "this function exhaustively dispatches one opcode family"
    )]
    use JVM::*;

    let instruction = match jvm_instruction {
        IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
            let definition = require_definition_id(definition)?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Read { array_ref, index };

            frame.push_value::<CATEGORY_1>(definition.into())?;
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Array(array_op),
            }
        }
        LALoad | DALoad => {
            let definition = require_definition_id(definition)?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Read { array_ref, index };
            frame.push_value::<CATEGORY_2>(definition.into())?;
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Array(array_op),
            }
        }
        IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
            let value = frame.pop_value::<CATEGORY_1>()?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            };

            RegisterInstruction::Effect(Expression::Array(array_op))
        }
        LAStore | DAStore => {
            let value = frame.pop_value::<CATEGORY_2>()?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            };
            RegisterInstruction::Effect(Expression::Array(array_op))
        }
        ANewArray(element_type) => {
            let definition = require_definition_id(definition)?;
            let count = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(definition.into())?;
            let array_op = ArrayOperation::New {
                element_type: element_type.clone().into(),
                length: count,
            };
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Array(array_op),
            }
        }
        NewArray(primitive_type) => {
            let definition = require_definition_id(definition)?;
            let count = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(definition.into())?;
            let array_op = ArrayOperation::New {
                element_type: FieldType::Base(*primitive_type),
                length: count,
            };
            RegisterInstruction::Definition {
                value: definition,
                expr: Expression::Array(array_op),
            }
        }
        MultiANewArray(element_type, dimension) => {
            let definition = require_definition_id(definition)?;
            let dimensions: Vec<_> = (0..*dimension)
                .map(|_| frame.pop_value::<CATEGORY_1>())
                .collect::<Result<_, _>>()?;
            frame.push_value::<CATEGORY_1>(definition.into())?;
            let expr = Expression::Array(ArrayOperation::NewMultiDim {
                element_type: element_type.clone().into(),
                dimensions,
            });
            RegisterInstruction::Definition {
                value: definition,
                expr,
            }
        }
        ArrayLength => {
            let definition = require_definition_id(definition)?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(definition.into())?;
            let expr = Expression::Array(ArrayOperation::Length { array_ref });
            RegisterInstruction::Definition {
                value: definition,
                expr,
            }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
