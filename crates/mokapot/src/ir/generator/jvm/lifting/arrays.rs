use crate::{
    ir::{
        expression::ArrayOperation,
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

pub(super) fn try_lift(
    jvm_instruction: &JVM,
    definition: Option<SsaValueId>,
    frame: &mut Frame<Value>,
) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
    use JVM::{
        AALoad, AAStore, ANewArray, ArrayLength, BALoad, BAStore, CALoad, CAStore, DALoad, DAStore,
        FALoad, FAStore, IALoad, IAStore, LALoad, LAStore, MultiANewArray, NewArray, SALoad,
        SAStore,
    };

    let instruction = match jvm_instruction {
        IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
            let value = require_definition_id(definition)?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let expr = ArrayOperation::Read { array_ref, index }.into();

            frame.push_value::<CATEGORY_1>(value.into())?;
            RegisterInstruction::Definition { value, expr }
        }
        LALoad | DALoad => {
            let value = require_definition_id(definition)?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let expr = ArrayOperation::Read { array_ref, index }.into();
            frame.push_value::<CATEGORY_2>(value.into())?;
            RegisterInstruction::Definition { value, expr }
        }
        IAStore | FAStore | AAStore | BAStore | CAStore | SAStore => {
            let value = frame.pop_value::<CATEGORY_1>()?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            }
            .into();
            RegisterInstruction::Effect(array_op)
        }
        LAStore | DAStore => {
            let value = frame.pop_value::<CATEGORY_2>()?;
            let index = frame.pop_value::<CATEGORY_1>()?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            let array_op = ArrayOperation::Write {
                array_ref,
                index,
                value,
            }
            .into();
            RegisterInstruction::Effect(array_op)
        }
        ANewArray(element_type) => {
            let value = require_definition_id(definition)?;
            let length = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = ArrayOperation::New {
                element_type: element_type.clone().into(),
                length,
            }
            .into();
            RegisterInstruction::Definition { value, expr }
        }
        NewArray(primitive_type) => {
            let value = require_definition_id(definition)?;
            let length = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = ArrayOperation::New {
                element_type: FieldType::Base(*primitive_type),
                length,
            }
            .into();
            RegisterInstruction::Definition { value, expr }
        }
        MultiANewArray(element_type, dimension) => {
            let value = require_definition_id(definition)?;
            let dimensions: Vec<_> = (0..*dimension)
                .map(|_| frame.pop_value::<CATEGORY_1>())
                .collect::<Result<_, _>>()?;
            frame.push_value::<CATEGORY_1>(value.into())?;
            let array_op = ArrayOperation::NewMultiDim {
                element_type: element_type.clone().into(),
                dimensions,
            }
            .into();
            RegisterInstruction::Definition {
                value,
                expr: array_op,
            }
        }
        ArrayLength => {
            let value = require_definition_id(definition)?;
            let array_ref = frame.pop_value::<CATEGORY_1>()?;
            frame.push_value::<CATEGORY_1>(value.into())?;
            let expr = ArrayOperation::Length { array_ref }.into();
            RegisterInstruction::Definition { value, expr }
        }
        _ => return Ok(None),
    };
    Ok(Some(instruction))
}
