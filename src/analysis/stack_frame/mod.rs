use std::{
    cmp::max,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
    fmt::{write, Display},
    iter::once,
    usize,
};

use itertools::Itertools;

use crate::elements::{
    instruction::{Instruction, MethodBody, ProgramCounter},
    ConstantValue, Method, MethodAccessFlags,
};
mod execution;
mod ir;

use self::ir::MokaInstruction;

use super::fixed_point::{self, FixedPointAnalyzer, FixedPointFact};

#[derive(PartialEq, Clone)]
pub struct StackFrame {
    pub max_locals: u16,
    pub max_stack: u16,
    pub local_variables: Vec<Option<FrameValue>>,
    pub operand_stack: Vec<FrameValue>,
    pub reachable_subroutines: BTreeSet<ProgramCounter>,
    pub preceding_kept_nodes: HashSet<ProgramCounter>,
}

impl StackFrame {
    pub(crate) fn pop_value(&mut self) -> Result<ValueRef, StackFrameError> {
        let value = self
            .operand_stack
            .pop()
            .ok_or(StackFrameError::StackUnderflow)?;
        match value {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Padding => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(crate) fn pop_padding(&mut self) -> Result<(), StackFrameError> {
        let value = self
            .operand_stack
            .pop()
            .ok_or(StackFrameError::StackUnderflow)?;
        match value {
            FrameValue::ValueRef(_) => Err(StackFrameError::ValueMismatch),
            FrameValue::Padding => Ok(()),
        }
    }

    pub(crate) fn push_value(&mut self, value: ValueRef) {
        self.operand_stack.push(FrameValue::ValueRef(value));
    }

    pub(crate) fn push_padding(&mut self) {
        self.operand_stack.push(FrameValue::Padding);
    }

    pub(crate) fn get_local(&self, idx: impl Into<usize>) -> Result<ValueRef, StackFrameError> {
        let frame_value = self
            .local_variables
            .get(idx.into())
            .unwrap()
            .clone()
            .ok_or(StackFrameError::LocalUnset)?;
        match frame_value {
            FrameValue::ValueRef(it) => Ok(it),
            FrameValue::Padding => Err(StackFrameError::ValueMismatch),
        }
    }

    pub(crate) fn set_local(&mut self, idx: impl Into<usize>, value: ValueRef) {
        self.local_variables
            .get_mut(idx.into())
            .expect("Out of index")
            .replace(FrameValue::ValueRef(value));
    }

    pub(crate) fn set_local_padding(&mut self, idx: impl Into<usize>) {
        self.local_variables
            .get_mut(idx.into())
            .expect("Out of index")
            .replace(FrameValue::Padding);
    }

    pub(crate) fn set_current_node(&mut self, pc: ProgramCounter) {
        self.preceding_kept_nodes.clear();
        self.preceding_kept_nodes.insert(pc);
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StackFrameError {
    #[error("Trying to pop an empty stack")]
    StackUnderflow,
    #[error("Expected a ValueRef but got Padding")]
    ValueMismatch,
    #[error("The local variable is not initialized")]
    LocalUnset,
}

pub struct StackFrameAnalyzer {
    code_map: HashMap<ProgramCounter, MokaInstruction>,
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
            locals[local_idx].replace(Identifier::This.into());
            local_idx += 1;
        }
        for i in 0..method.descriptor.parameters_types.len() {
            locals[local_idx].replace(Identifier::Arg(i as u8).into());
            local_idx += 1;
        }
        StackFrame {
            max_locals: body.max_locals,
            max_stack: body.max_stack,
            local_variables: locals,
            operand_stack: Vec::with_capacity(body.max_stack as usize),
            reachable_subroutines: BTreeSet::new(),
            preceding_kept_nodes: HashSet::new(),
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
                handler_frame
                    .operand_stack
                    .push(Identifier::CaughtException.into());
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
        let prededing_kept_nodes = self
            .preceding_kept_nodes
            .union(&other.preceding_kept_nodes)
            .cloned()
            .collect();

        Self {
            max_locals,
            max_stack,
            local_variables,
            operand_stack: stack,
            reachable_subroutines,
            preceding_kept_nodes: prededing_kept_nodes,
        }
    }
}

impl Default for StackFrameAnalyzer {
    fn default() -> Self {
        Self {
            code_map: Default::default(),
        }
    }
}

impl StackFrameAnalyzer {
    pub fn moka_ir(self, method: &Method) -> HashMap<ProgramCounter, MokaInstruction> {
        let mut self_mut = self;
        fixed_point::analyze(method, &mut self_mut);
        self_mut.code_map
    }
}

pub trait AbstractValue {}

#[derive(Debug, PartialEq, Eq, Hash, Clone, Copy)]
pub enum Identifier {
    Val(u16),
    This,
    Arg(u8),
    CaughtException,
}

impl Display for Identifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Identifier::*;
        match self {
            Val(idx) => write!(f, "v{}", idx),
            This => write!(f, "this"),
            Arg(idx) => write!(f, "arg{}", idx),
            CaughtException => write!(f, "exception"),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum ValueRef {
    Def(Identifier),
    Phi(HashSet<Identifier>),
}

impl Display for ValueRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ValueRef::Def(id) => write!(f, "{}", id),
            ValueRef::Phi(ids) => {
                write!(f, "Phi({})", ids.iter().map(|it| it.to_string()).join(", "))
            }
        }
    }
}

impl From<Identifier> for ValueRef {
    fn from(value: Identifier) -> Self {
        Self::Def(value)
    }
}

#[derive(Debug, PartialEq, Eq, Clone)]
pub enum FrameValue {
    ValueRef(ValueRef),
    Padding,
}

impl From<Identifier> for FrameValue {
    fn from(value: Identifier) -> Self {
        Self::ValueRef(ValueRef::Def(value))
    }
}

impl Display for FrameValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use FrameValue::*;
        match self {
            ValueRef(id) => write!(f, "{}", id),
            Padding => write!(f, "Padding"),
        }
    }
}

#[derive(Debug)]
pub enum Expression {
    Const(ConstantValue),
    ReturnAddress(ProgramCounter),
    Expr {
        instruction: Instruction,
        arguments: Vec<ValueRef>,
    },
}

impl Display for Expression {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use Expression::*;
        match self {
            Const(c) => write!(f, "{:?}", c),
            ReturnAddress(pc) => write!(f, "{:?}", pc),
            Expr {
                instruction,
                arguments,
            } => {
                write!(
                    f,
                    "{}({})",
                    instruction.name(),
                    arguments.iter().map(|it| it.to_string()).join(", ")
                )
            }
        }
    }
}

impl FrameValue {
    pub fn merge(x: Self, y: Self) -> Self {
        use ValueRef::*;
        match (x, y) {
            (lhs, rhs) if lhs == rhs => lhs,
            (FrameValue::ValueRef(lhs), FrameValue::ValueRef(rhs)) => {
                let result = match (lhs, rhs) {
                    (Def(id_x), Def(id_y)) => {
                        let mut values = HashSet::new();
                        values.insert(id_x);
                        values.insert(id_y);
                        Phi(values)
                    }
                    (Def(id_x), Phi(ids)) => Phi(ids.into_iter().chain(once(id_x)).collect()),
                    (Phi(ids), Def(id_y)) => Phi(ids.into_iter().chain(once(id_y)).collect()),
                    (Phi(ids_x), Phi(ids_y)) => Phi(ids_x.into_iter().chain(ids_y).collect()),
                };
                FrameValue::ValueRef(result)
            }
            _ => panic!(),
        }
    }
}
