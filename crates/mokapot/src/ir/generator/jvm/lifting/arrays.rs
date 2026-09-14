use crate::{
    ir::{
        expression::ArrayOperation,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{CATEGORY_1, CATEGORY_2, Frame},
                instruction::RegisterInstruction,
                subroutine_expansion::Location,
                symbolic_execution::{Executor, Value},
            },
        },
    },
    jvm::code::Instruction as JVM,
    types::field_type::FieldType,
};

impl Executor<'_> {
    pub(super) fn try_lift_arrays(
        &mut self,
        jvm_instruction: &JVM,
        location: Location,
        frame: &mut Frame<Value>,
    ) -> Result<Option<RegisterInstruction>, MokaIRBuildError> {
        use JVM::{
            AALoad, AAStore, ANewArray, ArrayLength, BALoad, BAStore, CALoad, CAStore, DALoad,
            DAStore, FALoad, FAStore, IALoad, IAStore, LALoad, LAStore, MultiANewArray, NewArray,
            SALoad, SAStore,
        };

        let instruction = match jvm_instruction {
            IALoad | FALoad | AALoad | BALoad | CALoad | SALoad => {
                let value = self.definition_id_at(location)?;
                let index = frame.pop_value::<CATEGORY_1>()?;
                let array_ref = frame.pop_value::<CATEGORY_1>()?;
                let expr = ArrayOperation::Read { array_ref, index }.into();

                frame.push_value::<CATEGORY_1>(value.into())?;
                RegisterInstruction::Definition { value, expr }
            }
            LALoad | DALoad => {
                let value = self.definition_id_at(location)?;
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
                let value = self.definition_id_at(location)?;
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
                let value = self.definition_id_at(location)?;
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
                let value = self.definition_id_at(location)?;
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
                let value = self.definition_id_at(location)?;
                let array_ref = frame.pop_value::<CATEGORY_1>()?;
                frame.push_value::<CATEGORY_1>(value.into())?;
                let expr = ArrayOperation::Length { array_ref }.into();
                RegisterInstruction::Definition { value, expr }
            }
            _ => return Ok(None),
        };
        Ok(Some(instruction))
    }
}
