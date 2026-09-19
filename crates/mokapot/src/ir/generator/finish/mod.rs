//! Finishes canonical SSA blocks into completed public `MokaIR`.

use crate::{
    ir::{
        BasicBlock, InstructionLocation, MokaIRMethod, Operation, Phi, PhiInput, SourceMap,
        Successor, Terminator, ValueDefinition, ValueId,
        generator::{
            draft::{DraftBlock, DraftMethod},
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
        blocks,
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

    let mut source_map = SourceMap::default();
    let blocks = blocks
        .into_iter()
        .map(|(id, block)| {
            let block = materialize_block(id, block, &mut source_map);
            (id, block)
        })
        .collect();

    let method = MokaIRMethod::new(
        method,
        MokaIRMethodParts {
            entry_block: entry,
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
        if let Some(value) = block.caught_exception {
            self.define(value, ValueDefinition::CaughtException(block_id))?;
        }
        for (index, phi) in block.phis.iter().enumerate() {
            let location = InstructionLocation::Phi {
                block: block_id,
                index,
            };
            self.define(phi.value, ValueDefinition::Instruction(location))?;
        }
        for (index, operation) in block.operations.iter().enumerate() {
            let Some(value) = operation.kind.def() else {
                continue;
            };
            let location = InstructionLocation::Operation {
                block: block_id,
                index,
            };
            self.define(value, ValueDefinition::Instruction(location))?;
        }
        Ok(())
    }
}

fn materialize_block(
    id: crate::ir::BlockId,
    block: DraftBlock,
    source_map: &mut SourceMap,
) -> BasicBlock {
    if let Some(origin) = block.terminator.origin {
        source_map.record_terminator(origin, id);
    }
    let phis = block
        .phis
        .into_iter()
        .map(|phi| Phi {
            value: phi.value,
            inputs: phi
                .inputs
                .into_iter()
                .map(|(predecessor, value)| PhiInput { predecessor, value })
                .collect(),
        })
        .collect();
    let operations = block
        .operations
        .into_iter()
        .enumerate()
        .map(|(index, operation)| {
            if let Some(origin) = operation.origin {
                source_map.record_operation(origin, id, index);
            }
            Operation {
                kind: operation.kind,
            }
        })
        .collect();
    let successors = block
        .terminator
        .successors
        .into_iter()
        .map(|successor| Successor {
            id: successor.id,
            target: successor.target,
            transfer: successor.transfer,
        })
        .collect();
    BasicBlock {
        caught_exception: block.caught_exception,
        phis,
        operations,
        terminator: Terminator {
            kind: block.terminator.kind,
            successors,
        },
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
            BlockId, OperationKind, TerminatorKind,
            expression::MathOperation,
            generator::{
                canonicalize,
                draft::{DraftBlock, DraftMethod, DraftOperation, DraftPhi, DraftTerminator},
            },
        },
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn eliminated_phi_leaves_a_hole_without_renumbering_live_values() {
        let block = BlockId::new(0);
        let parameter = ValueId::new(0);
        let eliminated_phi = ValueId::new(1);
        let result = ValueId::new(2);
        let draft = DraftMethod {
            entry: block,
            blocks: BTreeMap::from([(
                block,
                DraftBlock {
                    caught_exception: None,
                    phis: vec![DraftPhi {
                        value: eliminated_phi,
                        inputs: vec![(block, parameter)],
                    }],
                    operations: vec![DraftOperation {
                        kind: OperationKind::Definition {
                            value: result,
                            expr: MathOperation::Increment(eliminated_phi, 1).into(),
                        },
                        origin: None,
                    }],
                    terminator: DraftTerminator {
                        kind: TerminatorKind::Return(Some(result)),
                        successors: vec![],
                        origin: None,
                    },
                },
            )]),
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
        assert_eq!(ir.definition_of(eliminated_phi), None);
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
