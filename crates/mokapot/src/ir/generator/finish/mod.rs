//! Finishes canonical SSA blocks into completed public `MokaIR`.

use crate::{
    ir::{
        BasicBlock, BlockKind, BlockParameter, InstructionLocation, MethodEntry, MokaIRMethod,
        Operation, Successor, ValueDefinition, ValueId,
        generator::{
            draft::{DraftBlock, DraftEdge, DraftMethod},
            error::Error,
        },
        method::MokaIRMethodParts,
    },
    jvm::Method,
};

/// Constructs public wrappers and indexes from canonical draft IR.
pub(super) fn finish(method: &Method, draft: DraftMethod) -> Result<MokaIRMethod, Error> {
    let DraftMethod {
        entry,
        entry_arguments,
        blocks,
        source_map,
        this_value,
        parameter_values,
    } = draft;
    let mut state = FinishState::default();
    if let Some(value) = this_value {
        state.define(value, ValueDefinition::This)?;
    }
    for (index, &value) in parameter_values.iter().enumerate() {
        let index = u16::try_from(index)
            .map_err(|_| Error::internal("the method parameter index cannot be represented"))?;
        state.define(value, ValueDefinition::Parameter(index))?;
    }
    for (&id, block) in &blocks {
        state.define_block_values(id, block)?;
    }

    let blocks = blocks
        .into_iter()
        .map(|(id, block)| (id, materialize_block(block)))
        .collect();

    let method = MokaIRMethod::new(
        method,
        MokaIRMethodParts {
            entry: MethodEntry {
                target: entry,
                arguments: entry_arguments,
            },
            blocks,
            source_map,
            this_value,
            parameter_values,
            value_definitions: state.definitions,
        },
    );

    Ok(method)
}

impl FinishState {
    fn define_block_values(
        &mut self,
        block_id: crate::ir::BlockId,
        block: &DraftBlock,
    ) -> Result<(), Error> {
        if let BlockKind::LandingPad { exception: value } = block.kind {
            self.define(value, ValueDefinition::CaughtException(block_id))?;
        }
        for (index, parameter) in block.parameters.iter().enumerate() {
            let location = InstructionLocation::BlockParameter {
                block: block_id,
                index,
            };
            self.define(parameter.value, ValueDefinition::Instruction(location))?;
        }
        for (index, operation) in block.operations.iter().enumerate() {
            let Some(value) = operation.def() else {
                continue;
            };
            let location = InstructionLocation::Operation {
                block: block_id,
                index,
            };
            self.define(value, ValueDefinition::Instruction(location))?;
        }
        if let Some(value) = block.terminator.def() {
            self.define(
                value,
                ValueDefinition::Instruction(InstructionLocation::Terminator { block: block_id }),
            )?;
        }
        Ok(())
    }
}

fn materialize_block(block: DraftBlock) -> BasicBlock {
    let parameters = block
        .parameters
        .into_iter()
        .map(|it| BlockParameter { value: it.value })
        .collect();
    let operations = block
        .operations
        .into_iter()
        .map(|kind| Operation { kind })
        .collect();
    let terminator = block
        .terminator
        .map_arms(|successor: DraftEdge| Successor {
            id: successor.id,
            target: successor.target,
            arguments: successor.arguments,
            transfer: successor.transfer,
        })
        .map_operation(|kind| Operation { kind });
    BasicBlock {
        kind: block.kind,
        parameters,
        operations,
        terminator,
    }
}

#[derive(Default)]
struct FinishState {
    definitions: Vec<Option<ValueDefinition>>,
}

impl FinishState {
    fn define(&mut self, value: ValueId, definition: ValueDefinition) -> Result<(), Error> {
        let index = usize::try_from(value.index())
            .map_err(|_| Error::internal("the value index cannot be addressed"))?;
        let required_len = index
            .checked_add(1)
            .ok_or_else(|| Error::internal("the value index cannot be addressed"))?;
        if self.definitions.len() < required_len {
            self.definitions.resize(required_len, None);
        }
        if self.definitions[index].replace(definition).is_some() {
            return Err(Error::internal("a value identity has multiple definitions"));
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ir::{
            BlockId, EdgeId, OperationKind,
            control_flow::ControlTransfer,
            expression::MathOperation,
            generator::{
                canonicalize,
                draft::{DraftBlock, DraftEdge, DraftMethod, DraftParameter, DraftTerminator},
            },
        },
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn eliminated_parameter_leaves_a_hole_without_renumbering_live_values() {
        let block = BlockId::new(0);
        let parameter = ValueId::new(0);
        let eliminated_parameter = ValueId::new(1);
        let result = ValueId::new(2);
        let draft = DraftMethod {
            entry: block,
            entry_arguments: vec![parameter],
            blocks: BTreeMap::from([(
                block,
                DraftBlock {
                    kind: BlockKind::Code,
                    parameters: vec![DraftParameter {
                        value: eliminated_parameter,
                    }],
                    operations: vec![OperationKind::Definition {
                        value: result,
                        expr: MathOperation::Increment(eliminated_parameter, 1).into(),
                    }],
                    terminator: DraftTerminator::Goto {
                        target: DraftEdge {
                            id: EdgeId::new(0),
                            target: crate::ir::SuccessorTarget::Block(block),
                            arguments: vec![parameter],
                            transfer: ControlTransfer::Unconditional,
                        },
                    },
                },
            )]),
            source_map: crate::ir::SourceMap::default(),
            this_value: None,
            parameter_values: vec![parameter],
        };
        let mut draft = draft;
        canonicalize::canonicalize(&mut draft).unwrap();
        let method = crate::tests::method(
            [(0, Instruction::ILoad0), (1, Instruction::IReturn)],
            "(I)I",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );

        let ir = finish(&method, draft).unwrap();
        let completed_block = ir.block(block).unwrap();
        let operation = &completed_block.operations[0];

        assert_eq!(ir.parameter_values(), [parameter]);
        assert_eq!(operation.def(), Some(result));
        assert_eq!(operation.uses(), [parameter].into_iter().collect());
        assert_eq!(ir.definition_of(eliminated_parameter), None);
        assert_eq!(
            ir.definition_of(result),
            Some(ValueDefinition::Instruction(
                InstructionLocation::Operation { block, index: 0 }
            ))
        );
        assert!(
            operation
                .uses()
                .into_iter()
                .chain(completed_block.terminator.uses())
                .all(|value| ir.definition_of(value).is_some())
        );
        crate::ir::verify::verify(&ir).unwrap();
    }
}
