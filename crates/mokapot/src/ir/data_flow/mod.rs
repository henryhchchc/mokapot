//! Data flow analysis.

use std::collections::{BTreeSet, HashMap};

use super::{DefUseChain, InstructionId, MokaIRMethod, ValueDefinition, ValueId};

impl<'a> DefUseChain<'a> {
    /// Creates a new def-use graph from a method.
    #[must_use]
    pub fn new(method: &'a MokaIRMethod) -> Self {
        let mut defs = method.value_definitions().collect::<HashMap<_, _>>();
        let mut uses: HashMap<ValueId, BTreeSet<InstructionId>> = HashMap::new();

        for block in method.blocks() {
            for phi in block.phis() {
                defs.insert(phi.value(), ValueDefinition::Instruction(phi.id()));
                for input in phi.inputs() {
                    uses.entry(input.value()).or_default().insert(phi.id());
                }
            }
            for instruction in block.instructions() {
                if let Some(value) = instruction.def() {
                    defs.insert(value, ValueDefinition::Instruction(instruction.id()));
                }
                for value in instruction.uses() {
                    uses.entry(value).or_default().insert(instruction.id());
                }
            }
            for value in block.terminator().uses() {
                uses.entry(value)
                    .or_default()
                    .insert(block.terminator().id());
            }
        }

        Self { method, defs, uses }
    }

    /// Returns the definition of a value.
    #[must_use]
    pub fn defined_at(&self, value: ValueId) -> Option<ValueDefinition> {
        self.defs.get(&value).copied()
    }

    /// Returns the instruction locations where a value is used.
    #[must_use]
    pub fn used_at(&self, value: ValueId) -> BTreeSet<InstructionId> {
        self.uses.get(&value).cloned().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;
    use crate::{
        ir::{
            BasicBlock, BlockId, EdgeId, InstructionId, Phi, PhiInput, SourceMap, Successor,
            Terminator, TerminatorKind, control_flow::ControlTransfer,
        },
        jvm::method,
    };

    #[test]
    fn records_entry_phi_and_terminator_data_flow() {
        let this = ValueId::new(0);
        let parameter = ValueId::new(1);
        let caught = ValueId::new(2);
        let merged = ValueId::new(3);
        let entry_id = BlockId::new(0);
        let merge_id = BlockId::new(1);
        let handler_id = BlockId::new(2);
        let phi_id = InstructionId::new(1);
        let return_id = InstructionId::new(3);
        let phi = Phi::new(phi_id, merged, vec![PhiInput::new(entry_id, this)]);
        let entry = BasicBlock::new(
            entry_id,
            vec![],
            vec![],
            Terminator::new(
                InstructionId::new(0),
                TerminatorKind::Goto,
                vec![Successor::new(
                    EdgeId::new(0),
                    merge_id,
                    ControlTransfer::Unconditional,
                )],
            ),
        );
        let merge = BasicBlock::new(
            merge_id,
            vec![phi],
            vec![],
            Terminator::new(
                InstructionId::new(2),
                TerminatorKind::Goto,
                vec![Successor::new(
                    EdgeId::new(1),
                    handler_id,
                    ControlTransfer::Unconditional,
                )],
            ),
        );
        let handler = BasicBlock::new(
            handler_id,
            vec![],
            vec![],
            Terminator::new(return_id, TerminatorKind::Return(Some(parameter)), vec![]),
        );
        let method = MokaIRMethod::new(
            method::AccessFlags::empty(),
            "test".to_owned(),
            "(I)I".parse().unwrap(),
            "org/mokapot/Test".parse().unwrap(),
            entry_id,
            vec![entry, merge, handler],
            SourceMap::default(),
            Some(this),
            vec![parameter],
            BTreeMap::from([(handler_id, caught)]),
            vec![
                ValueDefinition::This,
                ValueDefinition::Parameter(0),
                ValueDefinition::CaughtException(handler_id),
                ValueDefinition::Instruction(phi_id),
            ],
        );

        let chain = DefUseChain::new(&method);

        assert_eq!(chain.defined_at(this), Some(ValueDefinition::This));
        assert_eq!(
            chain.defined_at(parameter),
            Some(ValueDefinition::Parameter(0))
        );
        assert_eq!(
            chain.defined_at(caught),
            Some(ValueDefinition::CaughtException(handler_id))
        );
        assert_eq!(
            chain.defined_at(merged),
            Some(ValueDefinition::Instruction(phi_id))
        );
        assert_eq!(chain.used_at(this), BTreeSet::from([phi_id]));
        assert_eq!(chain.used_at(parameter), BTreeSet::from([return_id]));
        assert!(chain.used_at(merged).is_empty());
        assert!(chain.used_at(caught).is_empty());
    }
}
