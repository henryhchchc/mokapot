//! Assigns final identities and assembles completed public `MokaIR`.

use std::collections::BTreeMap;

use crate::ir::TryMapValues;
use crate::{
    ir::{
        BasicBlock, EdgeId, InstructionId, MokaIRMethod, Operation, Phi, PhiInput, SourceMap,
        Successor, Terminator, ValueDefinition, ValueId,
        generator::{error::Error, identity::SsaValueId, ssa},
    },
    jvm::Method,
};

/// Emits final identities, blocks, and provenance from scalar SSA blocks.
pub(super) fn emit(method: &Method, ssa: ssa::Graph) -> Result<MokaIRMethod, Error> {
    let mut allocation = Allocation::default();
    let this_value = ssa
        .this_value
        .map(|value| allocation.value(value, ValueDefinition::This))
        .transpose()?;
    let parameter_values = ssa
        .parameter_values
        .into_iter()
        .enumerate()
        .map(|(index, value)| {
            let index = u16::try_from(index)
                .map_err(|_| Error::internal("the method parameter index cannot be represented"))?;
            allocation.value(value, ValueDefinition::Parameter(index))
        })
        .collect::<Result<_, _>>()?;

    let mut caught_exceptions = BTreeMap::new();
    let mut block_ids = Vec::with_capacity(ssa.blocks.len());
    for block in &ssa.blocks {
        let ids = allocation.allocate_block(block)?;
        if let Some(value) = ids.caught_exception {
            caught_exceptions.insert(block.id, value);
        }
        block_ids.push(ids);
    }

    let mut source_map = SourceMap::default();
    let mut blocks = Vec::with_capacity(ssa.blocks.len());
    for (block, ids) in ssa.blocks.into_iter().zip(block_ids) {
        blocks.push(materialize_block(
            block,
            ids,
            &mut allocation,
            &mut source_map,
        )?);
    }

    let method = MokaIRMethod::new(
        method,
        ssa.entry,
        blocks,
        source_map,
        this_value,
        parameter_values,
        caught_exceptions,
        allocation.definitions,
    );

    Ok(method)
}

struct BlockInstructions {
    caught_exception: Option<ValueId>,
    phis: Vec<InstructionId>,
    operations: Vec<InstructionId>,
    terminator: InstructionId,
}

impl Allocation {
    fn allocate_block(&mut self, block: &ssa::Block) -> Result<BlockInstructions, Error> {
        let caught_exception = block
            .caught_exception
            .map(|value| self.value(value, ValueDefinition::CaughtException(block.id)))
            .transpose()?;
        let phis = block
            .phis
            .iter()
            .map(|phi| {
                let id = self.instruction()?;
                self.value(phi.value, ValueDefinition::Instruction(id))?;
                Ok(id)
            })
            .collect::<Result<_, Error>>()?;
        let operations = block
            .operations
            .iter()
            .map(|(_, kind)| {
                let id = self.instruction()?;
                if let Some(value) = kind.def() {
                    self.value(value, ValueDefinition::Instruction(id))?;
                }
                Ok(id)
            })
            .collect::<Result<_, Error>>()?;
        Ok(BlockInstructions {
            caught_exception,
            phis,
            operations,
            terminator: self.instruction()?,
        })
    }
}

fn materialize_block(
    block: ssa::Block,
    ids: BlockInstructions,
    allocation: &mut Allocation,
    source_map: &mut SourceMap,
) -> Result<BasicBlock, Error> {
    let phis = block
        .phis
        .into_iter()
        .zip(ids.phis)
        .map(|(phi, id)| {
            Ok(Phi {
                id,
                value: allocation.resolve(phi.value)?,
                inputs: phi
                    .inputs
                    .into_iter()
                    .map(|(predecessor, value)| {
                        let value = allocation.resolve(value)?;
                        Ok(PhiInput { predecessor, value })
                    })
                    .collect::<Result<_, Error>>()?,
            })
        })
        .collect::<Result<_, Error>>()?;
    let operations = block
        .operations
        .into_iter()
        .zip(ids.operations)
        .map(|((pc, kind), id)| {
            source_map.insert(pc, id);
            let kind = kind.try_map_values(|value| allocation.resolve(value))?;
            Ok(Operation { id, kind })
        })
        .collect::<Result<_, Error>>()?;
    let successors = block
        .successors
        .into_iter()
        .map(|successor| {
            Ok(Successor {
                id: allocation.edge()?,
                target: successor.target,
                transfer: successor
                    .transfer
                    .try_map_values(|value| allocation.resolve(value))?,
            })
        })
        .collect::<Result<_, Error>>()?;
    if let Some(pc) = block.terminator_source {
        source_map.insert(pc, ids.terminator);
    }
    Ok(BasicBlock {
        id: block.id,
        phis,
        operations,
        terminator: Terminator {
            id: ids.terminator,
            kind: block
                .terminator
                .try_map_values(|value| allocation.resolve(value))?,
            successors,
        },
    })
}

#[derive(Default)]
struct Allocation {
    next_instruction: u32,
    next_edge: u32,
    values: Vec<Option<ValueId>>,
    definitions: Vec<ValueDefinition>,
}

impl Allocation {
    fn instruction(&mut self) -> Result<InstructionId, Error> {
        let id = InstructionId::new(self.next_instruction);
        self.next_instruction = self
            .next_instruction
            .checked_add(1)
            .ok_or_else(|| Error::internal("the instruction identity space is exhausted"))?;
        Ok(id)
    }

    fn edge(&mut self) -> Result<EdgeId, Error> {
        let id = EdgeId::new(self.next_edge);
        self.next_edge = self
            .next_edge
            .checked_add(1)
            .ok_or_else(|| Error::internal("the edge identity space is exhausted"))?;
        Ok(id)
    }

    fn value(
        &mut self,
        temporary: SsaValueId,
        definition: ValueDefinition,
    ) -> Result<ValueId, Error> {
        let value_index = u32::try_from(self.definitions.len())
            .ok()
            .filter(|index| *index < u32::MAX)
            .ok_or_else(|| Error::internal("the value identity space is exhausted"))?;
        let temporary = ssa_index(temporary)?;
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
        let value = ValueId::new(value_index);
        self.values[temporary] = Some(value);
        self.definitions.push(definition);
        Ok(value)
    }

    fn resolve(&self, value: SsaValueId) -> Result<ValueId, Error> {
        self.values
            .get(ssa_index(value)?)
            .and_then(|value| *value)
            .ok_or_else(|| Error::internal("a temporary value has no emitted definition"))
    }
}

fn ssa_index(value: SsaValueId) -> Result<usize, Error> {
    usize::try_from(value.index())
        .map_err(|_| Error::internal("a temporary value index cannot be addressed"))
}
