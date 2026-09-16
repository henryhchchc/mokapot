//! Register-form effects produced while lifting non-control bytecode.

use super::FrameValue;
use crate::ir::{expression::Expression, generator::identity::SsaValueId};

#[derive(Debug)]
pub(in crate::ir::generator) enum LiftedEffect {
    Erased,
    Definition {
        value: SsaValueId,
        expr: Expression<FrameValue>,
    },
    Effect(Expression<FrameValue>),
}
