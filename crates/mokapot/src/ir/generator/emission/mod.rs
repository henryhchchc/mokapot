//! Assigns final identities and assembles completed public `MokaIR`.

use super::ssa::{SsaBlock, SsaGraph};
use super::{
    BTreeMap, BasicBlock, EdgeId, InstructionId, MokaIRBuildError, MokaIRMethod, Operation, Phi,
    PhiInput, SourceMap, SsaValueId, Successor, Terminator, ValueDefinition, ValueId,
};
use crate::ir::TryMapValues;

/// Emits final identities, blocks, and provenance from fully lowered SSA.
pub(super) fn emit(
    method: &super::Method,
    ssa: SsaGraph,
) -> Result<MokaIRMethod, MokaIRBuildError> {
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
            let index = u16::try_from(index).map_err(|_| MokaIRBuildError::MalformedControlFlow)?;
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

    Ok(MokaIRMethod {
        access_flags: method.access_flags,
        name: method.name.clone(),
        descriptor: method.descriptor.clone(),
        owner: method.owner.clone(),
        entry_block: ssa.entry,
        blocks,
        source_map,
        this_value,
        parameter_values,
        caught_exceptions,
        value_definitions: allocation.definitions,
    })
}

struct BlockInstructions {
    caught_exception: Option<ValueId>,
    phis: Vec<InstructionId>,
    operations: Vec<InstructionId>,
    terminator: InstructionId,
}

impl Allocation {
    fn allocate_block(&mut self, block: &SsaBlock) -> Result<BlockInstructions, MokaIRBuildError> {
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
            .collect::<Result<_, MokaIRBuildError>>()?;
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
            .collect::<Result<_, MokaIRBuildError>>()?;
        Ok(BlockInstructions {
            caught_exception,
            phis,
            operations,
            terminator: self.instruction()?,
        })
    }
}

fn materialize_block(
    block: SsaBlock,
    ids: BlockInstructions,
    allocation: &mut Allocation,
    source_map: &mut SourceMap,
) -> Result<BasicBlock, MokaIRBuildError> {
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
                        Ok(PhiInput {
                            predecessor,
                            value: allocation.resolve(value)?,
                        })
                    })
                    .collect::<Result<_, MokaIRBuildError>>()?,
            })
        })
        .collect::<Result<_, MokaIRBuildError>>()?;
    let operations = block
        .operations
        .into_iter()
        .zip(ids.operations)
        .map(|((pc, kind), id)| {
            source_map.insert(pc, id);
            Ok(Operation {
                id,
                kind: kind.try_map_values(|value| allocation.resolve(value))?,
            })
        })
        .collect::<Result<_, MokaIRBuildError>>()?;
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
        .collect::<Result<_, MokaIRBuildError>>()?;
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
    values: BTreeMap<SsaValueId, ValueId>,
    definitions: Vec<ValueDefinition>,
}

impl Allocation {
    fn instruction(&mut self) -> Result<InstructionId, MokaIRBuildError> {
        let id = InstructionId::new(self.next_instruction);
        self.next_instruction = self
            .next_instruction
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }

    fn edge(&mut self) -> Result<EdgeId, MokaIRBuildError> {
        let id = EdgeId::new(self.next_edge);
        self.next_edge = self
            .next_edge
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }

    fn value(
        &mut self,
        temporary: SsaValueId,
        definition: ValueDefinition,
    ) -> Result<ValueId, MokaIRBuildError> {
        let index = u32::try_from(self.definitions.len())
            .ok()
            .filter(|index| *index < u32::MAX)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let value = ValueId::new(index);
        if self.values.insert(temporary, value).is_some() {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        self.definitions.push(definition);
        Ok(value)
    }

    fn resolve(&self, value: SsaValueId) -> Result<ValueId, MokaIRBuildError> {
        self.values
            .get(&value)
            .copied()
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }
}
