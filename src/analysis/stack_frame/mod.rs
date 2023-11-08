use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    iter::once,
    usize,
};

use crate::elements::{
    instruction::{Instruction, MethodBody, ProgramCounter},
    ConstantValue, Method, MethodAccessFlags,
};
mod execution;

use super::fixed_point::{self, FixedPointAnalyzer, FixedPointFact};

#[derive(PartialEq, Clone)]
pub struct StackFrame {
    pub max_locals: u16,
    pub max_stack: u16,
    pub local_variables: Vec<Option<FrameValue>>,
    pub operand_stack: Vec<FrameValue>,
    pub reachable_subroutines: BTreeSet<ProgramCounter>,
}

pub struct StackFrameAnalyzer {
    defs: HashMap<DefId, JavaAbstractValue>,
}


impl FixedPointAnalyzer<StackFrame> for StackFrameAnalyzer {
    fn entry_frame(&self, method: &Method) -> StackFrame {
        let body = method.body.as_ref().expect("TODO");
        let mut locals = Vec::with_capacity(body.max_locals as usize);
        for _ in 0..body.max_locals {
            locals.push(None);
        }
        let mut local_idx = 0;
        if !method.access_flags.contains(MethodAccessFlags::STATIC) {
            locals[local_idx].replace(FrameValue::Def(DefId::This));
            local_idx += 1;
        }
        for i in 0..method.descriptor.parameters_types.len() {
            locals[local_idx].replace(FrameValue::Def(DefId::Arg(i as u8)));
            local_idx += 1;
        }
        StackFrame {
            max_locals: body.max_locals,
            max_stack: body.max_stack,
            local_variables: locals,
            operand_stack: Vec::with_capacity(body.max_stack as usize),
            reachable_subroutines: BTreeSet::new(),
        }
    }

    fn execute_instruction(
        &mut self,
        body: &MethodBody,
        pc: ProgramCounter,
        insn: &Instruction,
        fact: &StackFrame,
    ) -> BTreeMap<ProgramCounter, StackFrame> {
        let mut frame = fact.clone();
        let mut dirty_pcs = BTreeMap::new();
        self.run_instruction(insn, pc, &mut frame);
        use Instruction::*;
        match insn {
            IfEq(target) | IfNe(target) | IfLt(target) | IfGe(target) | IfGt(target)
            | IfLe(target) | IfICmpEq(target) | IfICmpNe(target) | IfICmpLt(target)
            | IfICmpGe(target) | IfICmpGt(target) | IfICmpLe(target) | IfACmpEq(target)
            | IfACmpNe(target) | IfNull(target) | IfNonNull(target) => {
                let next_pc = body.next_pc_of(pc).expect("Cannot get next pc");
                dirty_pcs.insert(*target, frame.clone());
                dirty_pcs.insert(next_pc, frame.clone());
            }
            Goto(target) | GotoW(target) => {
                dirty_pcs.insert(*target, frame.clone());
            }
            TableSwitch {
                default,
                jump_targets,
                ..
            } => {
                jump_targets.iter().for_each(|it| {
                    dirty_pcs.insert(*it, frame.clone());
                });
                dirty_pcs.insert(*default, frame.clone());
            }
            LookupSwitch {
                default,
                match_targets,
            } => {
                match_targets.iter().for_each(|it| {
                    dirty_pcs.insert(it.1, frame.clone());
                });
                dirty_pcs.insert(*default, frame.clone());
            }
            Jsr(target) | JsrW(target) => {
                frame.reachable_subroutines.insert(*target);
                dirty_pcs.insert(*target, frame.clone());
            }
            AThrow => {}
            Ret(_) | WideRet(_) => frame.reachable_subroutines.iter().for_each(|it| {
                let next_pc = body.next_pc_of(*it).expect("Cannot get next pc");
                dirty_pcs.insert(next_pc, frame.clone());
            }),
            Return | AReturn | IReturn | LReturn | FReturn | DReturn => {}
            _ => {
                let next_pc = body.next_pc_of(pc).expect("Cannot get next pc");
                dirty_pcs.insert(next_pc, frame.clone());
            }
        }
        for handler in body.exception_table.iter() {
            if handler.covers(pc) {
                let mut handler_frame = frame.clone();
                handler_frame.operand_stack.clear();
                let def_id = DefId::Exception(handler.handler_pc);
                self.defs.insert(def_id, JavaAbstractValue::CaughtException);
                handler_frame.operand_stack.push(FrameValue::Def(def_id));
                dirty_pcs.insert(handler.handler_pc, handler_frame);
            }
        }

        dirty_pcs
    }
}

impl FixedPointFact for StackFrame {
    fn merge(&self, other: Self) -> Self {
        let mut other = other;

        let max_locals = max(self.max_locals, other.max_locals);
        let max_stack = max(self.max_stack, other.max_stack);
        let mut reachable_subroutines = self.reachable_subroutines.clone();
        reachable_subroutines.append(&mut other.reachable_subroutines);
        let mut local_variables = Vec::with_capacity(max_locals as usize);
        for i in 0..max_locals as usize {
            local_variables.insert(i, None);
            let self_loc = self.local_variables.get(i).cloned();
            let other_loc = other.local_variables.get(i).cloned();
            local_variables[i] = match (self_loc, other_loc) {
                (Some(x), Some(y)) => match (x, y) {
                    (Some(lhs), Some(rhs)) => Some(FrameValue::merge(lhs, rhs)),
                    (lhs, rhs) => lhs.or(rhs),
                },
                (x, y) => x
                    .or(y)
                    .expect("The local variable vec is not allocated correctly"),
            }
        }
        let mut stack = Vec::with_capacity(max_stack as usize);
        for i in 0..max(self.operand_stack.len(), other.operand_stack.len()) as usize {
            let self_loc = self.operand_stack.get(i).cloned();
            let other_loc = other.operand_stack.get(i).cloned();
            let stack_value = match (self_loc, other_loc) {
                (Some(x), Some(y)) => FrameValue::merge(x, y),
                (x, y) => x.or(y).expect("The stack vec is not allocated correctly"),
            };
            stack.push(stack_value);
        }

        Self {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: stack,
            reachable_subroutines,
        }
    }
}

impl Default for StackFrameAnalyzer {
    fn default() -> Self {
        Self { defs: Default::default() }
    }
}

impl StackFrameAnalyzer {
    pub fn definitions(self, method: &Method) -> HashMap<DefId, JavaAbstractValue> {
        let mut self_mut = self;
        fixed_point::analyze(method, &mut self_mut);
        self_mut.defs
    }
}

pub trait AbstractValue {}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum DefId {
    At(ProgramCounter),
    Exception(ProgramCounter),
    This,
    Arg(u8),
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrameValue {
    Def(DefId),
    Phi(HashSet<DefId>),
    Padding,
}


#[derive(Debug)]
pub enum JavaAbstractValue {
    Const(ConstantValue),
    ReturnAddress(ProgramCounter),
    Expression {
        instruction: Instruction,
        arguments: Vec<FrameValue>,
    },
    CaughtException,
}

impl FrameValue {
    pub fn merge(x: Self, y: Self) -> Self {
        use FrameValue::*;
        match (x, y) {
            (lhs, rhs) if lhs == rhs => lhs,
            (Def(id_x), Def(id_y)) => {
                let mut values = HashSet::new();
                values.insert(id_x);
                values.insert(id_y);
                Phi(values)
            },
            (Def(id_x), Phi(ids)) => Phi(ids.into_iter().chain(once(id_x)).collect()),
            (Phi(ids), Def(id_y)) => Phi(ids.into_iter().chain(once(id_y)).collect()),
            (Phi(ids_x), Phi(ids_y)) => Phi(ids_x.into_iter().chain(ids_y).collect()),
            (Padding, Padding) => Padding,
            _ => panic!(),
        }
    }
}
