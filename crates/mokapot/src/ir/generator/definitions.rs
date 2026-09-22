//! Derives the value-definition index for canonical SSA blocks.

use std::collections::HashMap;

use crate::ir::{BasicBlock, BlockId, BlockKind, InstructionLocation, ValueDefinition, ValueId};

/// Indexes the defining site of every SSA value in a completed method.
pub(super) fn index_definitions(
    this: Option<ValueId>,
    parameters: &[ValueId],
    blocks: &HashMap<BlockId, BasicBlock>,
) -> HashMap<ValueId, ValueDefinition> {
    let mut index = DefinitionIndex::default();
    if let Some(this) = this {
        index.define(this, ValueDefinition::This);
    }
    for (pos, &param) in parameters.iter().enumerate() {
        let pos = u16::try_from(pos).expect("a method declares fewer than 2^16 parameters");
        index.define(param, ValueDefinition::Parameter(pos));
    }
    for (&id, block) in blocks {
        index.define_block_values(id, block);
    }

    index.definitions
}

#[derive(Default)]
struct DefinitionIndex {
    definitions: HashMap<ValueId, ValueDefinition>,
}

impl DefinitionIndex {
    fn define_block_values(&mut self, id: BlockId, bb: &BasicBlock) {
        if let BlockKind::LandingPad { exception: value } = bb.kind {
            self.define(value, ValueDefinition::CaughtException(id));
        }
        for (index, parameter) in bb.parameters.iter().enumerate() {
            let location = InstructionLocation::BlockParameter { block: id, index };
            self.define(parameter.value, ValueDefinition::Instruction(location));
        }
        for (index, operation) in bb.operations.iter().enumerate() {
            if let Some(value) = operation.def() {
                let location = InstructionLocation::Operation { block: id, index };
                self.define(value, ValueDefinition::Instruction(location));
            }
        }
        if let Some(value) = bb.terminator.def() {
            let location = InstructionLocation::Terminator { block: id };
            self.define(value, ValueDefinition::Instruction(location));
        }
    }

    fn define(&mut self, value: ValueId, definition: ValueDefinition) {
        if self.definitions.insert(value, definition).is_some() {
            debug_assert!(false, "a value identity has multiple definitions");
        }
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::{
        ir::{NumericalId, test::prelude::*},
        jvm::ConstantValue::Null,
    };

    #[test]
    fn defines_this_then_parameters_in_descriptor_order() {
        let [this, first, second] = ids(0);

        let definitions = index_definitions(Some(this), &[first, second], &HashMap::new());

        let expected = HashMap::from([
            (this, ValueDefinition::This),
            (first, ValueDefinition::Parameter(0)),
            (second, ValueDefinition::Parameter(1)),
        ]);
        assert_eq!(definitions, expected);
    }

    #[test]
    fn defines_block_values_by_structural_location() {
        use InstructionLocation as Loc;
        use ValueDefinition::{CaughtException, Instruction};

        let [exception, param, defined, attempted] = ids(0);
        let block_id = BlockId::from_raw(0);
        let ops = [effect(Null), def(defined, Null)];
        let terminator = try_op(def(attempted, Null), Successor::Unwind, vec![]);
        let (block, body) = landing_pad(block_id, exception, [param], &ops, terminator);

        let expected = HashMap::from([
            (exception, CaughtException(block)),
            (param, Instruction(Loc::BlockParameter { block, index: 0 })),
            (defined, Instruction(Loc::Operation { block, index: 1 })),
            (attempted, Instruction(Loc::Terminator { block })),
        ]);
        let indexed = index_definitions(None, &[], &HashMap::from([(block, body)]));
        assert_eq!(indexed, expected);
    }
}
