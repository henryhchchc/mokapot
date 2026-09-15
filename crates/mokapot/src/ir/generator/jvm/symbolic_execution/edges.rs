//! Symbolic JVM edge construction.

use super::{Edge, Executor, NodeAddress, RegisterInstruction, Value};
use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, Value as PathValue},
        },
        expression::Condition,
        generator::{error::MokaIRBuildError, jvm::frame::Frame},
    },
    jvm::ConstantValue,
};

impl Executor<'_> {
    fn build_exception_edges(
        &mut self,
        addr: NodeAddress,
        incoming_frame: &Frame<Value>,
    ) -> Result<Vec<Edge>, MokaIRBuildError> {
        let pc = addr
            .source_pc()
            .ok_or(MokaIRBuildError::MalformedControlFlow)?;
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

    #[expect(
        clippy::too_many_lines,
        reason = "all control-flow forms are classified together"
    )]
    pub(crate) fn build_outgoing_edges(
        &mut self,
        addr: NodeAddress,
        incoming_frame: &Frame<Value>,
        normal_frame: Frame<Value>,
        instruction: &RegisterInstruction,
        can_throw_synchronously: bool,
    ) -> Result<Vec<Edge>, MokaIRBuildError> {
        use ControlTransfer::{Conditional, Normal, Unconditional};

        let edges = match instruction {
            RegisterInstruction::HandlerEntry => {
                let NodeAddress::Handler {
                    handler: handler_pc,
                    ..
                } = addr
                else {
                    return Err(MokaIRBuildError::MalformedControlFlow);
                };
                vec![Edge {
                    target: self.bytecode_addr_at(addr, handler_pc)?,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                }]
            }
            RegisterInstruction::Return(_) if can_throw_synchronously => {
                self.build_exception_edges(addr, incoming_frame)?
            }
            RegisterInstruction::Unwind | RegisterInstruction::Return(_) => Vec::new(),
            RegisterInstruction::Throw(_) => self.build_exception_edges(addr, incoming_frame)?,
            RegisterInstruction::Subroutine { target, .. } => {
                vec![Edge {
                    target: *target,
                    transfer: Unconditional,
                    target_frame: normal_frame,
                }]
            }
            RegisterInstruction::Definition { .. } | RegisterInstruction::Effect(_)
                if can_throw_synchronously =>
            {
                let mut edges = vec![Edge {
                    target: self.fallthrough_addr(addr)?,
                    transfer: Normal,
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
            } => {
                let condition: BooleanVariable<_> = condition.clone().into();
                vec![
                    Edge {
                        target: self.bytecode_addr_at(addr, *target)?,
                        transfer: Conditional(BranchGuard::of(condition.clone())),
                        target_frame: normal_frame.clone(),
                    },
                    Edge {
                        target: self.fallthrough_addr(addr)?,
                        transfer: Conditional(BranchGuard::of(!condition)),
                        target_frame: normal_frame,
                    },
                ]
            }
            RegisterInstruction::Switch {
                default,
                branches,
                match_value,
            } => {
                let mut edges = Vec::with_capacity(branches.len() + 1);
                for (&case, &target) in branches {
                    let value = PathValue::Constant(ConstantValue::Integer(case));
                    let condition =
                        BooleanVariable::Positive(Condition::Equal((*match_value).into(), value));
                    edges.push(Edge {
                        target: self.bytecode_addr_at(addr, target)?,
                        transfer: Conditional(BranchGuard::of(condition)),
                        target_frame: normal_frame.clone(),
                    });
                }
                let default_guard = branches
                    .keys()
                    .map(|case| {
                        BooleanVariable::Negative(Condition::Equal(
                            (*match_value).into(),
                            PathValue::Constant(ConstantValue::Integer(*case)),
                        ))
                    })
                    .collect();
                edges.push(Edge {
                    target: self.bytecode_addr_at(addr, *default)?,
                    transfer: Conditional(default_guard),
                    target_frame: normal_frame,
                });
                edges
            }
            RegisterInstruction::SubroutineReturn(value) => {
                let Value::ReturnAddress(address) = value else {
                    return Err(MokaIRBuildError::MalformedControlFlow);
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
    fn unwind_edges_erase_symbolic_values() {
        let method = method([(0, JvmInstruction::Nop)], "(I)V", vec![]);
        let mut executor = Executor::for_method(&method).expect("valid method");
        let frame = Frame::for_method_entry(
            &method.descriptor,
            1,
            0,
            None,
            &[Value::Ssa(SsaValueId::new(0))],
        )
        .expect("frame fits descriptor");

        let edges = executor
            .build_exception_edges(NodeAddress::entry(0.into()), &frame)
            .expect("valid exception edge");

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, NodeAddress::Unwind);
        assert!(matches!(edges[0].transfer, ControlTransfer::Unwind));
        assert!(edges[0].target_frame.iter_values().next().is_none());
    }
}
