use crate::ir::{
    expression::{Conversion, Expression, MathOperation},
    generator::{
        error::MokaIRBuildError,
        identity::SsaValueId,
        jvm::{
            frame::JvmStackFrame, instruction::RegisterInstruction,
            symbolic_execution::SymbolicValue,
        },
    },
};

#[inline]
pub(super) fn conversion_op<const OPERAND_SLOT: bool, const RESULT_SLOT: bool>(
    frame: &mut JvmStackFrame<SymbolicValue>,
    def: SsaValueId,
    conversion: impl FnOnce(SymbolicValue) -> Conversion<SymbolicValue>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let operand = frame.pop_value::<OPERAND_SLOT>()?;
    frame.push_value::<RESULT_SLOT>(def.into())?;
    Ok(RegisterInstruction::Definition {
        value: def,
        expr: Expression::Conversion(conversion(operand)),
    })
}

#[inline]
pub(super) fn binary_op_math<const SLOT: bool>(
    frame: &mut JvmStackFrame<SymbolicValue>,
    def_id: SsaValueId,
    math: impl FnOnce(SymbolicValue, SymbolicValue) -> MathOperation<SymbolicValue>,
) -> Result<RegisterInstruction, MokaIRBuildError> {
    let rhs = frame.pop_value::<SLOT>()?;
    let lhs = frame.pop_value::<SLOT>()?;
    frame.push_value::<SLOT>(def_id.into())?;

    let expr = Expression::Math(math(lhs, rhs));
    Ok(RegisterInstruction::Definition {
        value: def_id,
        expr,
    })
}
