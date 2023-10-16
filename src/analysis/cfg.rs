use crate::elements::{
    instruction::{MethodBody, ProgramCounter},
    references::ClassReference,
};

pub struct ControlFlowGraph<'b> {
    method_body: &'b MethodBody,
    edges: Vec<ControlFlowEdge>,
    entry: ProgramCounter,
    exits: Vec<ProgramCounter>,
}

impl<'b> ControlFlowGraph<'b> {
    pub fn new(method_body: &'b MethodBody) -> Self {
        let mut edges = Vec::new();
        let entry = ProgramCounter(0);
        let mut exits = Vec::new();
        for (pc, instruction) in method_body.instructions.iter() {
            todo!("Construct edges and exits")
        }
        Self {
            method_body,
            edges,
            entry,
            exits,
        }
    }
}

pub enum ControlFlowEdge {
    Execution {
        source: ProgramCounter,
        target: ProgramCounter,
    },
    Exception {
        source: ProgramCounter,
        target: ProgramCounter,
        exception: ClassReference,
    },
}
