//! JVM instruction-graph edge construction.

use std::collections::BTreeMap;

use super::{Edge, NodeAddress, NodeGraphBuilder, RegisterInstruction, Value};
use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, Value as PathValue},
        },
        expression::Condition,
        generator::{bytecode_analysis::jvm::Frame, error::Error},
    },
    jvm::{ConstantValue, code::ProgramCounter},
};

impl NodeGraphBuilder<'_> {
    fn build_exception_edges(
        &mut self,
        addr: NodeAddress,
        incoming_frame: &Frame<Value>,
    ) -> Result<Vec<Edge>, Error> {
        let pc = addr.source_pc().ok_or(Error::MalformedControlFlow)?;
        let handlers: Vec<_> = self
            .body()
            .exception_table
            .iter()
            .filter(|entry| entry.covers(pc))
            .cloned()
            .collect();
        let mut edges = Vec::with_capacity(handlers.len() + 1);
        let mut has_catch_all = false;
        for entry in handlers {
            let handler = self.exception_handler_addr(addr, entry.handler_pc)?;
            let caught = Value::Ssa(self.caught_exception_id_at(handler)?);
            edges.push(Edge {
                target: handler,
                transfer: ControlTransfer::Exception(entry.catch_type.clone()),
                target_frame: incoming_frame.exception_handler_frame(caught)?,
            });
            has_catch_all = entry
                .catch_type
                .as_ref()
                .is_none_or(|caught_type| caught_type.0.as_ref() == "java/lang/Throwable");
            if has_catch_all {
                break;
            }
        }
        if !has_catch_all {
            edges.push(Edge {
                target: self.unwind_addr()?,
                transfer: ControlTransfer::Unwind,
                target_frame: incoming_frame.clone().into_unwind_frame(),
            });
        }
        Ok(edges)
    }

    /// Builds the taken edge and its negated fallthrough edge for a conditional
    /// jump.
    fn build_conditional_jump_edges(
        &mut self,
        addr: NodeAddress,
        normal_frame: Frame<Value>,
        condition: &Condition<Value>,
        target: ProgramCounter,
    ) -> Result<Vec<Edge>, Error> {
        let condition: BooleanVariable<_> = condition.clone().into();
        Ok(vec![
            Edge {
                target: self.bytecode_addr_at(addr, target)?,
                transfer: ControlTransfer::Conditional(BranchGuard::of(condition.clone())),
                target_frame: normal_frame.clone(),
            },
            Edge {
                target: self.fallthrough_addr(addr)?,
                transfer: ControlTransfer::Conditional(BranchGuard::of(!condition)),
                target_frame: normal_frame,
            },
        ])
    }

    /// Builds the single unconditional edge from a handler entry to its guarded
    /// bytecode address.
    fn build_handler_entry_edges(
        &mut self,
        addr: NodeAddress,
        normal_frame: Frame<Value>,
    ) -> Result<Vec<Edge>, Error> {
        let NodeAddress::Handler {
            handler: handler_pc,
            ..
        } = addr
        else {
            return Err(Error::MalformedControlFlow);
        };
        Ok(vec![Edge {
            target: self.bytecode_addr_at(addr, handler_pc)?,
            transfer: ControlTransfer::Unconditional,
            target_frame: normal_frame,
        }])
    }

    /// Builds one guarded edge per switch arm, plus a default edge guarded by
    /// the negation of every arm condition.
    fn build_switch_edges(
        &mut self,
        addr: NodeAddress,
        normal_frame: Frame<Value>,
        match_value: Value,
        branches: &BTreeMap<i32, ProgramCounter>,
        default: ProgramCounter,
    ) -> Result<Vec<Edge>, Error> {
        let mut edges = Vec::with_capacity(branches.len() + 1);
        for (&case, &target) in branches {
            let value = PathValue::Constant(ConstantValue::Integer(case));
            let condition = BooleanVariable::Positive(Condition::Equal(match_value.into(), value));
            let switch_arm = Edge {
                target: self.bytecode_addr_at(addr, target)?,
                transfer: ControlTransfer::Conditional(BranchGuard::of(condition)),
                target_frame: normal_frame.clone(),
            };
            edges.push(switch_arm);
        }
        let default_arm = {
            let default_guard = branches
                .keys()
                .map(|case| {
                    let case_value = PathValue::Constant(ConstantValue::Integer(*case));
                    BooleanVariable::Negative(Condition::Equal(match_value.into(), case_value))
                })
                .collect();
            Edge {
                target: self.bytecode_addr_at(addr, default)?,
                transfer: ControlTransfer::Conditional(default_guard),
                target_frame: normal_frame,
            }
        };
        edges.push(default_arm);
        Ok(edges)
    }

    pub(crate) fn build_outgoing_edges(
        &mut self,
        addr: NodeAddress,
        incoming_frame: &Frame<Value>,
        normal_frame: Frame<Value>,
        instruction: &RegisterInstruction,
        can_throw_synchronously: bool,
    ) -> Result<Vec<Edge>, Error> {
        use ControlTransfer::Unconditional;

        let edges = match instruction {
            RegisterInstruction::HandlerEntry => {
                self.build_handler_entry_edges(addr, normal_frame)?
            }
            RegisterInstruction::Return(_) if can_throw_synchronously => {
                self.build_exception_edges(addr, incoming_frame)?
            }
            RegisterInstruction::Unwind | RegisterInstruction::Return(_) => Vec::new(),
            RegisterInstruction::Throw(_) => self.build_exception_edges(addr, incoming_frame)?,
            RegisterInstruction::Subroutine { target, .. } => {
                let edge = Edge {
                    target: *target,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                };
                vec![edge]
            }
            RegisterInstruction::Definition { .. } | RegisterInstruction::Effect(_)
                if can_throw_synchronously =>
            {
                // The did-not-throw outcome. It is unguarded like an ordinary
                // fallthrough, but `Node::can_throw_synchronously` keeps block
                // formation from eliding it.
                let mut edges = vec![Edge {
                    target: self.fallthrough_addr(addr)?,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                }];
                edges.extend(self.build_exception_edges(addr, incoming_frame)?);
                edges
            }
            RegisterInstruction::Erased
            | RegisterInstruction::Definition { .. }
            | RegisterInstruction::Effect(_) => {
                vec![Edge {
                    target: self.fallthrough_addr(addr)?,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                }]
            }
            RegisterInstruction::Jump {
                condition: None,
                target,
            } => vec![Edge {
                target: self.bytecode_addr_at(addr, *target)?,
                transfer: Unconditional,
                target_frame: normal_frame,
            }],
            RegisterInstruction::Jump {
                condition: Some(condition),
                target,
            } => self.build_conditional_jump_edges(addr, normal_frame, condition, *target)?,
            RegisterInstruction::Switch {
                default,
                branches,
                match_value,
            } => self.build_switch_edges(addr, normal_frame, *match_value, branches, *default)?,
            RegisterInstruction::SubroutineReturn(value) => {
                let Value::ReturnAddress(address) = value else {
                    return Err(Error::MalformedControlFlow);
                };
                vec![Edge {
                    target: self.return_from(addr, *address)?,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                }]
            }
        };
        Ok(edges)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::generator::{identity::SsaValueId, tests::method};
    use crate::jvm::code::Instruction as JvmInstruction;

    #[test]
    fn unwind_edges_erase_frame_values() {
        let method = method([(0, JvmInstruction::Nop)], "(I)V", vec![]);
        let mut builder = NodeGraphBuilder::for_method(&method).expect("valid method");
        let frame = Frame::for_method_entry(
            &method.descriptor,
            1,
            0,
            None,
            &[Value::Ssa(SsaValueId::new(0))],
        )
        .expect("frame fits descriptor");

        let edges = builder
            .build_exception_edges(NodeAddress::entry(0.into()), &frame)
            .expect("valid exception edge");

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, NodeAddress::Unwind);
        assert!(matches!(edges[0].transfer, ControlTransfer::Unwind));
        assert!(edges[0].target_frame.iter_values().next().is_none());
    }
}
