use std::collections::{BTreeMap, BTreeSet, HashSet};

use crate::{
    ir::{
        BasicBlock, EdgeId, InstructionId, MokaIRBuildError, MokaIRMethod, Operation,
        OperationKind, Successor, Terminator, TerminatorKind, ValueDefinition, ValueId,
        control_flow::ControlTransfer,
    },
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, MethodBody, ProgramCounter},
        method,
    },
};

pub(super) fn method<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> Method
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    let instructions = instructions
        .into_iter()
        .map(|(pc, inst)| (pc.into(), inst))
        .collect();
    Method {
        access_flags: method::AccessFlags::PUBLIC | method::AccessFlags::STATIC,
        name: "test".to_owned(),
        descriptor: descriptor.parse().unwrap(),
        owner: "org/mokapot/Test".parse().unwrap(),
        body: Some(MethodBody {
            max_stack: 4,
            max_locals: 4,
            instructions,
            exception_table,
            line_number_table: None,
            local_variable_table: None,
            stack_map_table: None,
            runtime_visible_type_annotations: vec![],
            runtime_invisible_type_annotations: vec![],
            other_attributes: vec![],
        }),
        exceptions: vec![],
        runtime_visible_annotations: vec![],
        runtime_invisible_annotations: vec![],
        runtime_visible_type_annotations: vec![],
        runtime_invisible_type_annotations: vec![],
        runtime_visible_parameter_annotations: vec![],
        runtime_invisible_parameter_annotations: vec![],
        annotation_default: None,
        parameters: vec![],
        is_synthetic: false,
        is_deprecated: false,
        signature: None,
        other_attributes: vec![],
    }
}

fn build(method: &Method) -> Result<MokaIRMethod, MokaIRBuildError> {
    MokaIRMethod::from_method(method)
}

mod blocks;
mod control_flow;
mod effects;
mod exceptions;
mod legacy;
mod phis;
