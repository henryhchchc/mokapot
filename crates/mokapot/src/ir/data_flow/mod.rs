//! Data flow analysis.

use std::collections::{BTreeSet, HashMap};

use super::{BlockId, InstructionId, MokaIRMethod, ValueDefinition, ValueId};

/// A location at which an SSA value is used.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UseSite {
    /// An ordinary instruction, terminator, or successor guard.
    Instruction(InstructionId),
    /// An input selected on entry from one predecessor block.
    PhiInput {
        /// The phi instruction consuming the value.
        phi: InstructionId,
        /// The predecessor selecting this input.
        predecessor: BlockId,
    },
}

impl UseSite {
    /// Returns the identified IR node containing this use.
    #[must_use]
    pub const fn instruction(self) -> InstructionId {
        match self {
            Self::Instruction(instruction)
            | Self::PhiInput {
                phi: instruction, ..
            } => instruction,
        }
    }
}

/// An owned, method-local index of scalar SSA definitions and uses.
///
/// Phi uses remain predecessor-sensitive through [`UseSite::PhiInput`].
#[derive(Debug)]
pub struct DefUseChain {
    defs: HashMap<ValueId, ValueDefinition>,
    uses: HashMap<ValueId, BTreeSet<UseSite>>,
}

impl DefUseChain {
    /// Creates a new def-use index from a method.
    #[must_use]
    pub fn new(method: &MokaIRMethod) -> Self {
        let defs = method.value_definitions().collect::<HashMap<_, _>>();
        let mut uses: HashMap<ValueId, BTreeSet<UseSite>> = HashMap::new();

        for block in method.blocks() {
            for phi in block.phis() {
                for input in phi.inputs() {
                    uses.entry(input.value())
                        .or_default()
                        .insert(UseSite::PhiInput {
                            phi: phi.id(),
                            predecessor: input.predecessor(),
                        });
                }
            }
            for operation in block.operations() {
                for value in operation.uses() {
                    uses.entry(value)
                        .or_default()
                        .insert(UseSite::Instruction(operation.id()));
                }
            }
            for value in block.terminator().uses() {
                uses.entry(value)
                    .or_default()
                    .insert(UseSite::Instruction(block.terminator().id()));
            }
        }

        Self { defs, uses }
    }

    /// Returns the definition of a value.
    #[must_use]
    pub fn definition_of(&self, value: ValueId) -> Option<ValueDefinition> {
        self.defs.get(&value).copied()
    }

    /// Iterates over the locations where a value is used in deterministic order.
    pub fn uses_of(&self, value: ValueId) -> impl Iterator<Item = UseSite> + '_ {
        self.uses
            .get(&value)
            .into_iter()
            .flat_map(|sites| sites.iter().copied())
    }
}

#[cfg(test)]
mod tests;
