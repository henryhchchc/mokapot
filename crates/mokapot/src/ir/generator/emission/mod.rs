//! Assembles completed public `MokaIR` from SSA blocks.

use std::collections::BTreeMap;

use crate::{
    ir::{
        BasicBlock, EdgeId, InstructionLocation, MokaIRMethod, Operation, Phi, PhiInput, SourceMap,
        Successor, Terminator, ValueDefinition, ValueId,
        generator::{error::Error, remap::RemapValues, ssa},
        method::MokaIRMethodParts,
    },
    jvm::Method,
};

/// Emits blocks and provenance from scalar SSA blocks.
pub(super) fn emit(
    method: &Method,
    ssa: ssa::SsaGraph,
    source_map: SourceMap,
) -> Result<MokaIRMethod, Error> {
    let ssa::SsaGraph {
        entry,
        blocks,
        this_value,
        parameter_values,
    } = ssa;
    let mut state = EmissionState::default();
    let this_value = this_value
        .map(|value| state.value(value, ValueDefinition::This))
        .transpose()?;
    let parameter_values = parameter_values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let index = u16::try_from(index)
                .map_err(|_| Error::internal("the method parameter index cannot be represented"))?;
            state.value(value, ValueDefinition::Parameter(index))
        })
        .collect::<Result<_, _>>()?;

    let mut block_allocations = blocks
        .iter()
        .map(|(&id, block)| state.allocate_block(id, block).map(|state| (id, state)))
        .collect::<Result<BTreeMap<_, _>, _>>()?;

    let blocks = blocks
        .into_iter()
        .map(|(id, block)| {
            let allocation = block_allocations
                .remove(&id)
                .ok_or_else(|| Error::internal("an SSA block has no instruction allocation"))?;
            materialize_block(block, allocation, &mut state).map(|block| (id, block))
        })
        .collect::<Result<_, Error>>()?;

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

#[derive(Clone, Copy)]
struct BlockAllocation {
    caught_exception: Option<ValueId>,
}

impl EmissionState {
    fn allocate_block(
        &mut self,
        block_id: crate::ir::BlockId,
        block: &ssa::Block,
    ) -> Result<BlockAllocation, Error> {
        let caught_exception = block
            .scalar
            .caught_exception
            .map(|value| self.value(value, ValueDefinition::CaughtException(block_id)))
            .transpose()?;
        block
            .phis
            .iter()
            .enumerate()
            .map(|(index, phi)| {
                let location = InstructionLocation::Phi {
                    block: block_id,
                    index,
                };
                self.value(phi.value, ValueDefinition::Instruction(location))?;
                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;
        block
            .scalar
            .operations
            .iter()
            .enumerate()
            .map(|(index, kind)| {
                let location = InstructionLocation::Operation {
                    block: block_id,
                    index,
                };
                if let Some(value) = kind.def() {
                    self.value(value, ValueDefinition::Instruction(location))?;
                }
                Ok(())
            })
            .collect::<Result<Vec<_>, Error>>()?;
        Ok(BlockAllocation { caught_exception })
    }
}

fn materialize_block(
    block: ssa::Block,
    allocation: BlockAllocation,
    state: &mut EmissionState,
) -> Result<BasicBlock, Error> {
    let phis = block
        .phis
        .into_iter()
        .map(|phi| {
            Ok(Phi {
                value: state.resolve(phi.value)?,
                inputs: phi
                    .inputs
                    .into_iter()
                    .map(|(predecessor, value)| {
                        let value = state.resolve(value)?;
                        Ok(PhiInput { predecessor, value })
                    })
                    .collect::<Result<_, Error>>()?,
            })
        })
        .collect::<Result<_, Error>>()?;
    let operations = block
        .scalar
        .operations
        .into_iter()
        .map(|mut kind| {
            kind.try_remap_values(&mut |value| state.resolve(value))?;
            Ok(Operation { kind })
        })
        .collect::<Result<_, Error>>()?;
    let successors = block
        .scalar
        .successors
        .into_iter()
        .map(|(target, mut transfer)| {
            transfer.try_remap_values(&mut |value| state.resolve(value))?;
            Ok(Successor {
                id: state.edge()?,
                target,
                transfer,
            })
        })
        .collect::<Result<_, Error>>()?;
    let mut terminator = block.scalar.terminator;
    terminator.try_remap_values(&mut |value| state.resolve(value))?;
    Ok(BasicBlock {
        caught_exception: allocation.caught_exception,
        phis,
        operations,
        terminator: Terminator {
            kind: terminator,
            successors,
        },
    })
}

#[derive(Default)]
struct EmissionState {
    next_edge: u32,
    values: Vec<Option<ValueId>>,
    definitions: Vec<ValueDefinition>,
}

impl EmissionState {
    fn edge(&mut self) -> Result<EdgeId, Error> {
        let id = EdgeId::new(self.next_edge);
        self.next_edge = self
            .next_edge
            .checked_add(1)
            .ok_or_else(|| Error::internal("the edge identity space is exhausted"))?;
        Ok(id)
    }

    fn value(&mut self, temporary: ValueId, definition: ValueDefinition) -> Result<ValueId, Error> {
        let emitted_index = u32::try_from(self.definitions.len())
            .ok()
            .filter(|index| *index < u32::MAX)
            .ok_or_else(|| Error::internal("the value identity space is exhausted"))?;
        let temporary = value_index(temporary)?;
        let required_len = temporary
            .checked_add(1)
            .ok_or_else(|| Error::internal("the temporary value index cannot be addressed"))?;
        if self.values.len() < required_len {
            self.values.resize(required_len, None);
        }
        if self.values[temporary].is_some() {
            return Err(Error::internal(
                "a temporary value identity has multiple definitions",
            ));
        }
        let value = ValueId::new(emitted_index);
        self.values[temporary] = Some(value);
        self.definitions.push(definition);
        Ok(value)
    }

    fn resolve(&self, value: ValueId) -> Result<ValueId, Error> {
        self.values
            .get(value_index(value)?)
            .and_then(|value| *value)
            .ok_or_else(|| Error::internal("a temporary value has no emitted definition"))
    }
}

fn value_index(value: ValueId) -> Result<usize, Error> {
    usize::try_from(value.index())
        .map_err(|_| Error::internal("a temporary value index cannot be addressed"))
}
