//! Shared JVM successor construction.

use crate::{
    ir::{
        control_flow::{
            ControlTransfer,
            path_condition::{BooleanVariable, BranchGuard, Value},
        },
        expression::Condition,
        generator::{
            error::MokaIRBuildError,
            jvm::{
                frame::{Entry, JvmStackFrame},
                instruction::RegisterInstruction,
                normalization::Location,
                symbolic_execution::{JvmSymbolicExecutor, SymbolicJvmEdge, SymbolicValue},
            },
        },
    },
    jvm::ConstantValue,
};

fn build_exception_successors(
    executor: &mut JvmSymbolicExecutor<'_>,
    location: Location,
    incoming_frame: &JvmStackFrame<SymbolicValue>,
) -> Result<Vec<SymbolicJvmEdge>, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let handlers = executor
        .body()
        .exception_table
        .iter()
        .filter(|entry| entry.covers(pc))
        .cloned()
        .collect::<Vec<_>>();
    let mut successors = Vec::with_capacity(handlers.len() + 1);
    let mut catches_all = false;
    for entry in handlers {
        let handler = executor.handler_location(location, entry.handler_pc)?;
        let caught = SymbolicValue::Value(executor.caught_exception_at(handler)?);
        successors.push(SymbolicJvmEdge {
            target: handler,
            transfer: ControlTransfer::Exception(entry.catch_type.clone()),
            target_frame: incoming_frame.same_locals_1_stack_item_frame(Entry::Value(caught)),
        });
        catches_all = entry
            .catch_type
            .as_ref()
            .is_none_or(|caught_type| caught_type.0.as_ref() == "java/lang/Throwable");
        if catches_all {
            break;
        }
    }
    if !catches_all {
        successors.push(SymbolicJvmEdge {
            target: executor.unwind_location()?,
            transfer: ControlTransfer::Unwind,
            target_frame: incoming_frame
                .same_locals_empty_stack_frame()
                .erase_values(),
        });
    }
    Ok(successors)
}

#[expect(
    clippy::too_many_lines,
    reason = "all control-flow forms are classified together"
)]
pub(crate) fn build_successors(
    executor: &mut JvmSymbolicExecutor<'_>,
    location: Location,
    incoming_frame: &JvmStackFrame<SymbolicValue>,
    post_frame: JvmStackFrame<SymbolicValue>,
    instruction: &RegisterInstruction,
    can_throw_synchronously: bool,
) -> Result<Vec<SymbolicJvmEdge>, MokaIRBuildError> {
    use ControlTransfer::{Conditional, Normal, Unconditional};

    Ok(match instruction {
        RegisterInstruction::HandlerEntry => {
            let Location::Handler { handler_pc, .. } = location else {
                return Err(MokaIRBuildError::MalformedControlFlow);
            };
            vec![SymbolicJvmEdge {
                target: executor.target_location(location, handler_pc)?,
                transfer: Unconditional,
                target_frame: post_frame,
            }]
        }
        RegisterInstruction::Return(_) if can_throw_synchronously => {
            build_exception_successors(executor, location, incoming_frame)?
        }
        RegisterInstruction::Unwind | RegisterInstruction::Return(_) => Vec::new(),
        RegisterInstruction::Throw(_) => {
            build_exception_successors(executor, location, incoming_frame)?
        }
        RegisterInstruction::Subroutine { target, .. } => {
            vec![SymbolicJvmEdge {
                target: *target,
                transfer: Unconditional,
                target_frame: post_frame,
            }]
        }
        RegisterInstruction::Definition { .. } | RegisterInstruction::Effect(_)
            if can_throw_synchronously =>
        {
            let mut successors = vec![SymbolicJvmEdge {
                target: executor.next_location(location)?,
                transfer: Normal,
                target_frame: post_frame,
            }];
            successors.extend(build_exception_successors(
                executor,
                location,
                incoming_frame,
            )?);
            successors
        }
        RegisterInstruction::Erased
        | RegisterInstruction::Definition { .. }
        | RegisterInstruction::Effect(_) => {
            vec![SymbolicJvmEdge {
                target: executor.next_location(location)?,
                transfer: Unconditional,
                target_frame: post_frame,
            }]
        }
        RegisterInstruction::Jump {
            condition: None,
            target,
        } => vec![SymbolicJvmEdge {
            target: executor.target_location(location, *target)?,
            transfer: Unconditional,
            target_frame: post_frame,
        }],
        RegisterInstruction::Jump {
            condition: Some(condition),
            target,
        } => {
            let condition: BooleanVariable<_> = condition.clone().into();
            vec![
                SymbolicJvmEdge {
                    target: executor.target_location(location, *target)?,
                    transfer: Conditional(BranchGuard::of(condition.clone())),
                    target_frame: post_frame.same_frame(),
                },
                SymbolicJvmEdge {
                    target: executor.next_location(location)?,
                    transfer: Conditional(BranchGuard::of(!condition)),
                    target_frame: post_frame,
                },
            ]
        }
        RegisterInstruction::Switch {
            default,
            branches,
            match_value,
        } => {
            let mut successors = Vec::with_capacity(branches.len() + 1);
            for (&case, &target) in branches {
                let value = Value::Constant(ConstantValue::Integer(case));
                let condition =
                    BooleanVariable::Positive(Condition::Equal((*match_value).into(), value));
                successors.push(SymbolicJvmEdge {
                    target: executor.target_location(location, target)?,
                    transfer: Conditional(BranchGuard::of(condition)),
                    target_frame: post_frame.same_frame(),
                });
            }
            let default_guard = branches
                .keys()
                .map(|case| {
                    BooleanVariable::Negative(Condition::Equal(
                        (*match_value).into(),
                        Value::Constant(ConstantValue::Integer(*case)),
                    ))
                })
                .collect();
            successors.push(SymbolicJvmEdge {
                target: executor.target_location(location, *default)?,
                transfer: Conditional(default_guard),
                target_frame: post_frame,
            });
            successors
        }
        RegisterInstruction::SubroutineReturn(value) => {
            let SymbolicValue::ReturnAddress(address) = value else {
                return Err(MokaIRBuildError::MalformedControlFlow);
            };
            vec![SymbolicJvmEdge {
                target: executor.return_from(location, *address)?,
                transfer: Unconditional,
                target_frame: post_frame,
            }]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::generator::{identity::SsaValueId, tests::method};
    use crate::jvm::code::Instruction as JvmInstruction;

    #[test]
    fn unwind_edges_erase_symbolic_values() {
        let method = method([(0.into(), JvmInstruction::Nop)], "(I)V", vec![]);
        let mut analyzer = JvmSymbolicExecutor::for_method(&method).expect("valid method");
        let frame = JvmStackFrame::with_inputs(
            &method.descriptor,
            1,
            0,
            None,
            &[SymbolicValue::Value(SsaValueId::new(0))],
        )
        .expect("frame fits descriptor");

        let edges = build_exception_successors(&mut analyzer, Location::entry(0.into()), &frame)
            .expect("valid exception edge");

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, Location::Unwind);
        assert!(matches!(edges[0].transfer, ControlTransfer::Unwind));
        assert!(edges[0].target_frame.values().next().is_none());
    }
}
