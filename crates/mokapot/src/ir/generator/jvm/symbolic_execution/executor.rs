use std::collections::{BTreeMap, BTreeSet};

use crate::{
    ir::generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            NodeAddress,
            frame::Frame,
            instruction::RegisterInstruction,
            lifting::{fallibility::FallibilityContext, successors::build_outgoing_edges},
            subroutine::{Expander, ReturnAddress},
            symbolic_execution::fact::{Cfg, Node, Value},
            symbolic_execution::solver,
        },
    },
    jvm::{
        Method,
        code::{MethodBody, ProgramCounter},
        method,
    },
};

/// Mutable state used only while performing symbolic execution.
pub(crate) struct Executor<'method> {
    body: &'method MethodBody,
    fallibility: FallibilityContext,
    subroutine_expander: Expander,
    definition_ids: BTreeMap<NodeAddress, SsaValueId>,
    caught_exception_ids: BTreeMap<NodeAddress, SsaValueId>,
    value_id_allocator: ValueIdAllocator,
    receiver_value: Option<SsaValueId>,
    parameter_values: Vec<SsaValueId>,
    entry_addr: NodeAddress,
    initial_frame: Frame<Value>,
}

impl<'method> Executor<'method> {
    pub fn execute_addr(
        &mut self,
        addr: NodeAddress,
        incoming_frame: Frame<Value>,
    ) -> Result<Node, MokaIRBuildError> {
        let (instruction, outgoing_edges) = match addr {
            NodeAddress::Handler { .. } => {
                let instruction = RegisterInstruction::HandlerEntry;
                let normal_frame = incoming_frame.clone();
                let outgoing_edges = build_outgoing_edges(
                    self,
                    addr,
                    &incoming_frame,
                    normal_frame,
                    &instruction,
                    false,
                )?;
                (instruction, outgoing_edges)
            }
            NodeAddress::Unwind => (RegisterInstruction::Unwind, Vec::new()),
            NodeAddress::Bytecode { pc, .. } => {
                let mut normal_frame = incoming_frame.clone();
                let jvm_instruction = self
                    .body
                    .instruction_at(pc)
                    .ok_or(MokaIRBuildError::MalformedControlFlow)?
                    .clone();
                let can_throw_synchronously =
                    self.fallibility.is_synchronously_fallible(&jvm_instruction);
                let instruction =
                    self.lift_register_instruction(&jvm_instruction, addr, &mut normal_frame)?;
                let outgoing_edges = build_outgoing_edges(
                    self,
                    addr,
                    &incoming_frame,
                    normal_frame,
                    &instruction,
                    can_throw_synchronously,
                )?;
                (instruction, outgoing_edges)
            }
        };

        Ok(Node {
            incoming_frame,
            instruction,
            outgoing_edges,
            caught_exception_value: self.caught_exception_ids.get(&addr).copied(),
        })
    }

    pub const fn body(&self) -> &MethodBody {
        self.body
    }

    pub fn for_method(method: &'method Method) -> Result<Self, MokaIRBuildError> {
        let body = method.body.as_ref().ok_or(MokaIRBuildError::NoMethodBody)?;
        let (first_pc, _) = body
            .instructions
            .entry_point()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let mut value_id_allocator = ValueIdAllocator::default();
        let receiver_value = (!method.access_flags.contains(method::AccessFlags::STATIC))
            .then(|| value_id_allocator.new_value_id())
            .transpose()?;
        let parameter_values: Vec<SsaValueId> = method
            .descriptor
            .parameters_types
            .iter()
            .map(|_| value_id_allocator.new_value_id())
            .collect::<Result<_, _>>()?;
        let frame_parameters = parameter_values
            .iter()
            .copied()
            .map(Value::Ssa)
            .collect::<Vec<_>>();
        let initial_frame = Frame::for_method_entry(
            &method.descriptor,
            body.max_locals,
            body.max_stack,
            receiver_value.map(Value::Ssa),
            &frame_parameters,
        )?;
        let entry_addr = NodeAddress::entry(first_pc);
        let executor = Self {
            body,
            fallibility: FallibilityContext::for_method(method),
            subroutine_expander: Expander::new(first_pc),
            definition_ids: BTreeMap::new(),
            caught_exception_ids: BTreeMap::new(),
            value_id_allocator,
            receiver_value,
            parameter_values,
            entry_addr,
            initial_frame,
        };
        Ok(executor)
    }

    pub fn execute(mut self) -> Result<Cfg, MokaIRBuildError> {
        let nodes = self.execute_reachable_addrs()?;
        let merge_identities = nodes
            .values()
            .flat_map(|node| node.incoming_frame.iter_values())
            .filter_map(|value| match value {
                Value::Merged(identity) => Some(identity.to_owned()),
                Value::Ssa(_) | Value::ReturnAddress(_) | Value::Invalid => None,
            })
            .collect::<BTreeSet<_>>();
        let phi_values = merge_identities
            .into_iter()
            .map(|identity| self.new_value_id().map(|value| (identity, value)))
            .collect::<Result<_, _>>()?;
        Ok(Cfg {
            entry_addr: self.entry_addr,
            initial_frame: self.initial_frame,
            nodes,
            phi_values,
            receiver_value: self.receiver_value,
            parameter_values: self.parameter_values,
        })
    }

    pub fn execute_reachable_addrs(
        &mut self,
    ) -> Result<BTreeMap<NodeAddress, Node>, MokaIRBuildError> {
        let entry_addr = self.entry_addr;
        let initial_frame = self.initial_frame.clone();
        solver::execute_to_fixpoint(self, entry_addr, initial_frame)
    }

    pub fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        self.value_id_allocator.new_value_id()
    }

    pub fn definition_id_at(&mut self, addr: NodeAddress) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(addr, NodeAddress::Bytecode { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.definition_ids.get(&addr) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.definition_ids.insert(addr, id);
        Ok(id)
    }

    pub fn caught_exception_id_at(
        &mut self,
        addr: NodeAddress,
    ) -> Result<SsaValueId, MokaIRBuildError> {
        if !matches!(addr, NodeAddress::Handler { .. }) {
            return Err(MokaIRBuildError::MalformedControlFlow);
        }
        if let Some(&id) = self.caught_exception_ids.get(&addr) {
            return Ok(id);
        }
        let id = self.new_value_id()?;
        self.caught_exception_ids.insert(addr, id);
        Ok(id)
    }

    pub fn next_program_counter(
        &self,
        pc: ProgramCounter,
    ) -> Result<ProgramCounter, MokaIRBuildError> {
        self.body
            .instructions
            .next_pc_of(&pc)
            .ok_or(MokaIRBuildError::MalformedControlFlow)
    }

    pub fn fallthrough_addr(&mut self, addr: NodeAddress) -> Result<NodeAddress, MokaIRBuildError> {
        let pc = addr
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        let context = addr
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.subroutine_expander
            .bytecode_addr(self.next_program_counter(pc)?, context)
    }

    pub fn bytecode_addr_at(
        &mut self,
        addr: NodeAddress,
        target: ProgramCounter,
    ) -> Result<NodeAddress, MokaIRBuildError> {
        let context = addr
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.subroutine_expander.bytecode_addr(target, context)
    }

    pub fn exception_handler_addr(
        &mut self,
        addr: NodeAddress,
        handler: ProgramCounter,
    ) -> Result<NodeAddress, MokaIRBuildError> {
        let context = addr
            .context()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        self.subroutine_expander.handler_addr(handler, context)
    }

    pub fn unwind_addr(&mut self) -> Result<NodeAddress, MokaIRBuildError> {
        self.subroutine_expander.register_addr(NodeAddress::Unwind)
    }

    pub fn enter_subroutine(
        &mut self,
        addr: NodeAddress,
        target: ProgramCounter,
        continuation: ProgramCounter,
    ) -> Result<(NodeAddress, ReturnAddress), MokaIRBuildError> {
        self.subroutine_expander
            .enter_subroutine(addr, target, continuation)
    }

    pub fn return_from(
        &mut self,
        addr: NodeAddress,
        address: ReturnAddress,
    ) -> Result<NodeAddress, MokaIRBuildError> {
        self.subroutine_expander.return_from(addr, address)
    }
}

#[derive(Debug, Default)]
pub(super) struct ValueIdAllocator {
    pub(super) next_value_idx: u32,
}

impl ValueIdAllocator {
    fn new_value_id(&mut self) -> Result<SsaValueId, MokaIRBuildError> {
        let id = SsaValueId::new(self.next_value_idx);
        self.next_value_idx = self
            .next_value_idx
            .checked_add(1)
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
        Ok(id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ir::generator::{jvm::NodeAddress, tests::method},
        jvm::code::Instruction as JvmInstruction,
    };

    #[test]
    fn reprocessing_loop_allocates_identities_only_for_definitions() {
        let method = method(
            [
                (0, JvmInstruction::IConst0),
                (1, JvmInstruction::IStore0),
                (2, JvmInstruction::ILoad0),
                (3, JvmInstruction::IConst1),
                (4, JvmInstruction::IAdd),
                (5, JvmInstruction::IStore0),
                (6, JvmInstruction::Goto(2.into())),
            ],
            "()V",
            vec![],
        );
        let mut executor = Executor::for_method(&method).expect("valid method");
        executor.execute_reachable_addrs().expect("valid loop");

        assert_eq!(executor.definition_ids.len(), 3);
        assert_eq!(executor.value_id_allocator.next_value_idx, 3);

        for k in [0, 3, 4] {
            assert!(
                executor
                    .definition_ids
                    .contains_key(&NodeAddress::entry(k.into()))
            );
        }
        for k in [1, 2, 5, 6] {
            assert!(
                !executor
                    .definition_ids
                    .contains_key(&NodeAddress::entry(k.into()))
            );
        }
    }
}
