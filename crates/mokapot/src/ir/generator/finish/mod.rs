//! Finishes canonical SSA blocks into completed public `MokaIR`.

use crate::{
    ir::{
        BasicBlock, InstructionLocation, MokaIRMethod, Operation, Phi, PhiInput, SourceMap,
        Successor, Terminator, ValueDefinition, ValueId,
        generator::{canonicalize, error::Error},
        method::MokaIRMethodParts,
    },
    jvm::Method,
};

/// Finishes blocks and provenance from canonical scalar SSA blocks.
pub(super) fn finish(
    method: &Method,
    graph: canonicalize::CanonicalGraph,
    source_map: SourceMap,
) -> Result<MokaIRMethod, Error> {
    let canonicalize::CanonicalGraph {
        entry,
        blocks,
        this_value,
        parameter_values,
    } = graph;
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
        block: &canonicalize::Block,
    ) -> Result<(), Error> {
        if let Some(value) = block.scalar.caught_exception {
            self.define(value, ValueDefinition::CaughtException(block_id))?;
        }
        for (index, phi) in block.phis.iter().enumerate() {
            let location = InstructionLocation::Phi {
                block: block_id,
                index,
            };
            self.define(phi.value, ValueDefinition::Instruction(location))?;
        }
        for (index, kind) in block.scalar.operations.iter().enumerate() {
            let Some(value) = kind.def() else {
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

fn materialize_block(block: canonicalize::Block) -> BasicBlock {
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
        .scalar
        .operations
        .into_iter()
        .map(|kind| Operation { kind })
        .collect();
    let successors = block
        .scalar
        .successors
        .into_iter()
        .map(|successor| Successor {
            id: successor.id,
            target: successor.target,
            transfer: successor.transfer,
        })
        .collect();
    BasicBlock {
        caught_exception: block.scalar.caught_exception,
        phis,
        operations,
        terminator: Terminator {
            kind: block.scalar.terminator,
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
            generator::bytecode_analysis::{PhiCandidate, ScalarBlock, ScalarGraph},
        },
        jvm::{code::Instruction, method::AccessFlags},
    };

    #[test]
    fn eliminated_phi_leaves_a_hole_without_renumbering_live_values() {
        let block = BlockId::new(0);
        let parameter = ValueId::new(0);
        let eliminated_phi = ValueId::new(1);
        let result = ValueId::new(2);
        let scalar = ScalarGraph {
            entry: block,
            blocks: BTreeMap::from([(
                block,
                ScalarBlock {
                    caught_exception: None,
                    operations: vec![OperationKind::Definition {
                        value: result,
                        expr: MathOperation::Increment(eliminated_phi, 1).into(),
                    }],
                    terminator: TerminatorKind::Return(Some(result)),
                    successors: vec![],
                },
            )]),
            phi_candidates: BTreeMap::from([(
                eliminated_phi,
                PhiCandidate {
                    placement: block,
                    inputs: vec![(block, parameter)],
                },
            )]),
            this_value: None,
            parameter_values: vec![parameter],
        };
        let graph = canonicalize::canonicalize(scalar).unwrap();
        let method = crate::tests::method(
            [(0, Instruction::ILoad0), (1, Instruction::IReturn)],
            "(I)I",
            vec![],
            AccessFlags::PUBLIC | AccessFlags::STATIC,
        );

        let ir = finish(&method, graph, SourceMap::default()).unwrap();
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
