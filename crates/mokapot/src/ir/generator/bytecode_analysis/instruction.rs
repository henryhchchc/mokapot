//! Register-form instructions used while lifting structural blocks.

use super::Value;
use crate::{
    ir::{
        expression::{Condition, Expression},
        generator::identity::SsaValueId,
    },
    jvm::code::ProgramCounter,
};

#[derive(Debug)]
pub(crate) enum RegisterInstruction {
    Erased,
    Definition {
        value: SsaValueId,
        expr: Expression<Value>,
    },
    Effect(Expression<Value>),
    Jump {
        condition: Option<Condition<Value>>,
    },
    Switch {
        match_value: Value,
    },
    Return(Option<Value>),
    Throw(Value),
    Subroutine {
        value: SsaValueId,
        continuation: ProgramCounter,
    },
    SubroutineReturn(Value),
}
