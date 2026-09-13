//! Shared JVM outgoing-edge semantics.

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

fn exception_edges(
    semantics: &mut JvmSymbolicExecutor<'_>,
    location: Location,
    pre_frame: &JvmStackFrame<SymbolicValue>,
) -> Result<Vec<SymbolicJvmEdge>, MokaIRBuildError> {
    let pc = location
        .source_pc()
        .ok_or(MokaIRBuildError::MalformedControlFlow)?;
    let entries = semantics
        .body()
        .exception_table
        .iter()
        .filter(|entry| entry.covers(pc))
        .cloned()
        .collect::<Vec<_>>();
    let mut outgoing = Vec::with_capacity(entries.len() + 1);
    let mut exhaustive = false;
    for entry in entries {
        let handler = semantics.handler_location(location, entry.handler_pc)?;
        let caught = SymbolicValue::Value(semantics.caught_exception_at(handler)?);
        outgoing.push(SymbolicJvmEdge {
            target: handler,
            transfer: ControlTransfer::Exception(entry.catch_type.clone()),
            target_frame: pre_frame.same_locals_1_stack_item_frame(Entry::Value(caught)),
        });
        exhaustive = entry
            .catch_type
            .as_ref()
            .is_none_or(|caught_type| caught_type.0.as_ref() == "java/lang/Throwable");
        if exhaustive {
            break;
        }
    }
    if !exhaustive {
        outgoing.push(SymbolicJvmEdge {
            target: semantics.unwind_location()?,
            transfer: ControlTransfer::Unwind,
            target_frame: pre_frame.same_locals_empty_stack_frame().erase_values(),
        });
    }
    Ok(outgoing)
}

#[expect(
    clippy::too_many_lines,
    reason = "all control-flow forms are classified together"
)]
pub(crate) fn outgoing_from(
    semantics: &mut JvmSymbolicExecutor<'_>,
    location: Location,
    pre_frame: &JvmStackFrame<SymbolicValue>,
    normal_frame: JvmStackFrame<SymbolicValue>,
    instruction: &RegisterInstruction,
    fallible: bool,
) -> Result<Vec<SymbolicJvmEdge>, MokaIRBuildError> {
    use ControlTransfer::{Conditional, Normal, Unconditional};

    Ok(match instruction {
        RegisterInstruction::HandlerEntry => {
            let Location::Handler { handler_pc, .. } = location else {
                return Err(MokaIRBuildError::MalformedControlFlow);
            };
            vec![SymbolicJvmEdge {
                target: semantics.target_location(location, handler_pc)?,
                transfer: Unconditional,
                target_frame: normal_frame,
            }]
        }
        RegisterInstruction::Return(_) if fallible => {
            exception_edges(semantics, location, pre_frame)?
        }
        RegisterInstruction::Unwind | RegisterInstruction::Return(_) => Vec::new(),
        RegisterInstruction::Throw(_) => exception_edges(semantics, location, pre_frame)?,
        RegisterInstruction::Subroutine { target, .. } => {
            vec![SymbolicJvmEdge {
                target: *target,
                transfer: Unconditional,
                target_frame: normal_frame,
            }]
        }
        RegisterInstruction::Definition { .. } | RegisterInstruction::Effect(_) if fallible => {
            let mut outgoing = vec![SymbolicJvmEdge {
                target: semantics.next_location(location)?,
                transfer: Normal,
                target_frame: normal_frame,
            }];
            outgoing.extend(exception_edges(semantics, location, pre_frame)?);
            outgoing
        }
        RegisterInstruction::Erased
        | RegisterInstruction::Definition { .. }
        | RegisterInstruction::Effect(_) => {
            vec![SymbolicJvmEdge {
                target: semantics.next_location(location)?,
                transfer: Unconditional,
                target_frame: normal_frame,
            }]
        }
        RegisterInstruction::Jump {
            condition: None,
            target,
        } => vec![SymbolicJvmEdge {
            target: semantics.target_location(location, *target)?,
            transfer: Unconditional,
            target_frame: normal_frame,
        }],
        RegisterInstruction::Jump {
            condition: Some(condition),
            target,
        } => {
            let condition: BooleanVariable<_> = condition.clone().into();
            vec![
                SymbolicJvmEdge {
                    target: semantics.target_location(location, *target)?,
                    transfer: Conditional(BranchGuard::of(condition.clone())),
                    target_frame: normal_frame.same_frame(),
                },
                SymbolicJvmEdge {
                    target: semantics.next_location(location)?,
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
            let mut outgoing = Vec::with_capacity(branches.len() + 1);
            for (&case, &target) in branches {
                let value = Value::Constant(ConstantValue::Integer(case));
                let condition =
                    BooleanVariable::Positive(Condition::Equal((*match_value).into(), value));
                outgoing.push(SymbolicJvmEdge {
                    target: semantics.target_location(location, target)?,
                    transfer: Conditional(BranchGuard::of(condition)),
                    target_frame: normal_frame.same_frame(),
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
            outgoing.push(SymbolicJvmEdge {
                target: semantics.target_location(location, *default)?,
                transfer: Conditional(default_guard),
                target_frame: normal_frame,
            });
            outgoing
        }
        RegisterInstruction::SubroutineReturn(value) => {
            let SymbolicValue::ReturnAddress(address) = value else {
                return Err(MokaIRBuildError::MalformedControlFlow);
            };
            vec![SymbolicJvmEdge {
                target: semantics.return_from(location, *address)?,
                transfer: Unconditional,
                target_frame: normal_frame,
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

        let edges = exception_edges(&mut analyzer, Location::entry(0.into()), &frame)
            .expect("valid exception edge");

        assert_eq!(edges.len(), 1);
        assert_eq!(edges[0].target, Location::Unwind);
        assert!(matches!(edges[0].transfer, ControlTransfer::Unwind));
        assert!(edges[0].target_frame.values().next().is_none());
    }
}
