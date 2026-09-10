use std::collections::{BTreeMap, HashSet};

use super::*;
use crate::jvm::code::{ExceptionTableEntry, Instruction, InstructionList};

pub(super) fn method(
    instructions: impl IntoIterator<Item = (ProgramCounter, Instruction)>,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> Method {
    Method {
        access_flags: method::AccessFlags::PUBLIC | method::AccessFlags::STATIC,
        name: "test".to_owned(),
        descriptor: descriptor.parse().unwrap(),
        owner: "org/mokapot/Test".parse().unwrap(),
        body: Some(MethodBody {
            max_stack: 4,
            max_locals: 4,
            instructions: InstructionList::from_iter(instructions),
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
