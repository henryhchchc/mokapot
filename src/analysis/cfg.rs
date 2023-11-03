use std::collections::HashSet;

use crate::elements::{
    instruction::{Instruction, MethodBody, ProgramCounter},
    references::ClassReference,
};

pub struct ControlFlowGraph<'b> {
    method_body: &'b MethodBody,
    edges: HashSet<ControlFlowEdge>,
    entry: ProgramCounter,
    exits: HashSet<ProgramCounter>,
}

impl<'b> ControlFlowGraph<'b> {
    pub fn new(method_body: &'b MethodBody) -> Self {
        let mut edges = HashSet::new();
        let entry = ProgramCounter::from(0);
        let mut exits = HashSet::new();
        let mut insn_iter = method_body.instructions.iter().peekable();
        while let Some((pc, insn)) = insn_iter.next() {
            use Instruction::*;
            match insn {
                IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
                | IfLe(target) | IfNull(target) | IfNonNull(target) | IfACmpEq(target)
                | IfACmpNe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
                | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) => {
                    edges.insert(ControlFlowEdge::Execution {
                        source: pc.clone(),
                        target: target.clone(),
                    });
                    if let Some((next_pc, _next_insn)) = insn_iter.peek() {
                        edges.insert(ControlFlowEdge::Execution {
                            source: pc.clone(),
                            target: next_pc.clone(),
                        });
                    }
                }
                Goto(target) | GotoW(target) => {
                    edges.insert(ControlFlowEdge::Execution {
                        source: pc.clone(),
                        target: target.clone(),
                    });
                }
                Return | AReturn | DReturn | FReturn | IReturn | LReturn => {
                    exits.insert(pc.clone());
                }
                TableSwitch {
                    default,
                    jump_targets,
                    ..
                } => {
                    jump_targets.into_iter().for_each(|target| {
                        edges.insert(ControlFlowEdge::Execution {
                            source: pc.clone(),
                            target: target.clone(),
                        });
                    });
                    edges.insert(ControlFlowEdge::Execution {
                        source: pc.clone(),
                        target: default.clone(),
                    });
                }
                LookupSwitch {
                    default,
                    match_targets,
                } => {
                    match_targets.into_iter().for_each(|(_, target)| {
                        edges.insert(ControlFlowEdge::Execution {
                            source: pc.clone(),
                            target: target.clone(),
                        });
                    });
                    edges.insert(ControlFlowEdge::Execution {
                        source: pc.clone(),
                        target: default.clone(),
                    });
                }
                Jsr(target) | JsrW(target) => {
                    edges.insert(ControlFlowEdge::Execution {
                        source: pc.clone(),
                        target: target.clone(),
                    });
                }
                Ret(_) => {
                    // TODO: Implement ret
                }
                AThrow => {}
                _ => {
                    if let Some((next_pc, _next_insn)) = insn_iter.peek() {
                        edges.insert(ControlFlowEdge::Execution {
                            source: pc.clone(),
                            target: next_pc.clone(),
                        });
                    }
                }
            }
            method_body.exception_table.iter().for_each(|e| {
                if e.covers(*pc) {
                    edges.insert(ControlFlowEdge::Exception {
                        source: *pc,
                        target: e.handler_pc,
                        catch_type: e.catch_type.clone(),
                    });
                }
            });
        }
        Self {
            method_body,
            edges,
            entry,
            exits,
        }
    }
}

#[derive(Hash, PartialEq, Eq)]
pub enum ControlFlowEdge {
    Execution {
        source: ProgramCounter,
        target: ProgramCounter,
    },
    Exception {
        source: ProgramCounter,
        target: ProgramCounter,
        catch_type: Option<ClassReference>,
    },
}
