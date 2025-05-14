//! Backend for generating LLVM IR for usage with tools provided by the LLVM
//! infrastructure.

use inkwell::{
    AddressSpace, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    module::Module,
    values::{BasicValueEnum, IntValue},
};

use crate::{
    ir::{
        Identifier, MokaInstruction, Operand,
        expression::{Condition, Expression, MathOperation},
    },
    jvm::{ConstantValue, code::ProgramCounter},
};
use utils::{get_or_insert_basic_block_ordered, upcast_to_u64};

mod intrinsics;
mod utils;

/// Trait representing a struct that can be lowered into LLVM IR.
// TODO(Derppening): Determine if this trait can be used for all lower-able constructs.
pub trait IRLowering {
    /// Lowers the LLVM IR representation of this struct and inserts it into the
    /// [`Module`].
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>>;

    /// Lowers an equality operation and inserts it into the [`Builder`].
    fn lower_eq_op<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        negated: bool,
    ) -> IntValue<'ctx> {
        let Some(lhs) = lhs.lower(module, builder, pc) else {
            panic!("Expect LHS operand {lhs:?} to lower to a BasicValue")
        };
        let Some(rhs) = rhs.lower(module, builder, pc) else {
            panic!("Expect RHS operand {rhs:?} to lower to a BasicValue")
        };

        let predicate = if negated {
            IntPredicate::NE
        } else {
            IntPredicate::EQ
        };

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                builder.build_int_compare(predicate, lhs, rhs, "").unwrap()
            }

            (BasicValueEnum::PointerValue(lhs), BasicValueEnum::PointerValue(rhs)) => {
                let ctx = module.get_context();

                // TODO(Derppening): Should we be assuming 32-bit pointer types?
                let lhs = builder.build_ptr_to_int(lhs, ctx.i32_type(), "").unwrap();
                let rhs = builder.build_ptr_to_int(rhs, ctx.i32_type(), "").unwrap();

                builder.build_int_compare(predicate, lhs, rhs, "").unwrap()
            }

            (_, _) => {
                panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or PointerValue")
            }
        }
    }

    /// Lowers an integer comparison operation and inserts it into the [`Builder`].
    fn lower_cmp_op<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> IntValue<'ctx> {
        let Some(BasicValueEnum::IntValue(lhs)) = lhs.lower(module, builder, pc) else {
            panic!("Expect LHS operand {lhs:?} to lower to an IntValue")
        };
        let Some(BasicValueEnum::IntValue(rhs)) = rhs.lower(module, builder, pc) else {
            panic!("Expect RHS operand {rhs:?} to lower to a BasicValue")
        };

        builder.build_int_compare(llvm_cmpop, lhs, rhs, "").unwrap()
    }

    /// Lowers a null check and inserts it into the [`Builder`].
    fn lower_null_check<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
        operand: &Operand,
        negated: bool,
    ) -> IntValue<'ctx> {
        let Some(BasicValueEnum::PointerValue(operand)) = operand.lower(module, builder, pc) else {
            panic!("Expect {operand:?} to lower to a PointerValue")
        };

        if negated {
            builder.build_is_not_null(operand, "").unwrap()
        } else {
            builder.build_is_null(operand, "").unwrap()
        }
    }

    /// Lowers a compare-with-zero operation and inserts it into the [`Builder`].
    ///
    /// Effectively inserts `{llvm_cmpop} {operand}, 0`.
    fn lower_cmp_zero_op<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
        operand: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> IntValue<'ctx> {
        let Some(BasicValueEnum::IntValue(operand)) = operand.lower(module, builder, pc) else {
            panic!("Expect {operand:?} to lower to an IntValue")
        };

        builder
            .build_int_compare(llvm_cmpop, operand, operand.get_type().const_zero(), "")
            .unwrap()
    }
}

impl IRLowering for MokaInstruction {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        let func_val = builder
            .get_insert_block()
            .and_then(BasicBlock::get_parent)
            .unwrap();
        let this_bb = get_or_insert_basic_block_ordered(module.get_context(), func_val, pc);

        // If the previous BB is not terminated, add a jmp to this BB
        if builder
            .get_insert_block()
            .map(BasicBlock::get_terminator)
            .is_none()
        {
            builder.build_unconditional_branch(this_bb).unwrap();
        }

        builder.position_at_end(this_bb);

        match self {
            MokaInstruction::Nop => intrinsics::invoke_donothing(module, builder),
            MokaInstruction::Jump { condition, target } => {
                let target_bb =
                    get_or_insert_basic_block_ordered(module.get_context(), func_val, *target);

                if let Some(condition) = condition {
                    let Some(BasicValueEnum::IntValue(condition)) =
                        condition.lower(module, builder, pc)
                    else {
                        panic!("Expect {condition:?} to lower to an IntValue")
                    };

                    let current_bb = builder.get_insert_block().unwrap();
                    let context = current_bb.get_context();

                    let then_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.then"));
                    let else_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.else"));
                    let cont_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.cont"));

                    builder
                        .build_conditional_branch(condition, then_bb, else_bb)
                        .unwrap();

                    builder.position_at_end(then_bb);
                    builder.build_unconditional_branch(target_bb).unwrap();

                    builder.position_at_end(else_bb);
                    builder.build_unconditional_branch(cont_bb).unwrap();

                    builder.position_at_end(cont_bb);
                } else {
                    builder.build_unconditional_branch(target_bb).unwrap();
                }
            }

            MokaInstruction::Return(operand) => {
                if let Some(operand) = operand {
                    let Some(operand) = operand.lower(module, builder, pc) else {
                        panic!("Expect {operand:?} to lower to a BasicValue")
                    };

                    builder.build_return(Some(&operand)).unwrap();
                } else {
                    builder.build_return(None).unwrap();
                }
            }

            _ => todo!("Unimplemented lowering for {self}"),
        }

        None
    }
}

impl IRLowering for Condition {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        Some(
            match self {
                Condition::Equal(lhs, rhs) => {
                    self.lower_eq_op(module, builder, pc, lhs, rhs, false)
                }
                Condition::NotEqual(lhs, rhs) => {
                    self.lower_eq_op(module, builder, pc, lhs, rhs, true)
                }

                Condition::LessThan(lhs, rhs) => {
                    self.lower_cmp_op(module, builder, pc, lhs, rhs, IntPredicate::SLT)
                }
                Condition::LessThanOrEqual(lhs, rhs) => {
                    self.lower_cmp_op(module, builder, pc, lhs, rhs, IntPredicate::SLE)
                }
                Condition::GreaterThan(lhs, rhs) => {
                    self.lower_cmp_op(module, builder, pc, lhs, rhs, IntPredicate::SGT)
                }
                Condition::GreaterThanOrEqual(lhs, rhs) => {
                    self.lower_cmp_op(module, builder, pc, lhs, rhs, IntPredicate::SGE)
                }

                Condition::IsNull(operand) => {
                    self.lower_null_check(module, builder, pc, operand, false)
                }
                Condition::IsNotNull(operand) => {
                    self.lower_null_check(module, builder, pc, operand, true)
                }

                Condition::IsZero(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::EQ)
                }
                Condition::IsNonZero(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::NE)
                }
                Condition::IsPositive(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::SGT)
                }
                Condition::IsNegative(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::SLT)
                }
                Condition::IsNonNegative(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::SGE)
                }
                Condition::IsNonPositive(operand) => {
                    self.lower_cmp_zero_op(module, builder, pc, operand, IntPredicate::SLE)
                }
            }
            .into(),
        )
    }
}

impl IRLowering for ConstantValue {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        _builder: &Builder<'ctx>,
        _: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        let ctx = module.get_context();

        Some(match self {
            ConstantValue::Null => ctx.ptr_type(AddressSpace::default()).const_null().into(),
            ConstantValue::Integer(v) => ctx.i32_type().const_int(upcast_to_u64(*v), true).into(),
            ConstantValue::Float(v) => ctx.f32_type().const_float(f64::from(*v)).into(),
            ConstantValue::Long(v) => ctx.i64_type().const_int(upcast_to_u64(*v), true).into(),
            ConstantValue::Double(v) => ctx.f64_type().const_float(*v).into(),

            _ => todo!("Unimplemented lowering for {self}"),
        })
    }
}

impl IRLowering for Expression {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        Some(match self {
            Expression::Const(value) => value.lower(module, builder, pc).unwrap(),
            Expression::Math(op) => op.lower(module, builder, pc).unwrap(),

            _ => todo!("Unimplemented lowering for {self}"),
        })
    }
}

impl IRLowering for Identifier {
    fn lower<'ctx>(
        &self,
        _module: &Module<'ctx>,
        _builder: &Builder<'ctx>,
        _pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        todo!("Unimplemented lowering for {self}")
    }
}

impl IRLowering for Operand {
    fn lower<'ctx>(
        &self,
        _module: &Module<'ctx>,
        _builder: &Builder<'ctx>,
        _pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        todo!("Unimplemented lowering for {self}")
    }
}

impl IRLowering for MathOperation {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        Some(match self {
            MathOperation::Add(lhs, rhs) => {
                let Some(lhs) = lhs.lower(module, builder, pc) else {
                    panic!("Expect LHS operand {lhs:?} to lower to a BasicValue")
                };
                let Some(rhs) = rhs.lower(module, builder, pc) else {
                    panic!("Expect RHS operand {rhs:?} to lower to a BasicValue")
                };

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                        builder.build_int_add(lhs, rhs, "").unwrap().into()
                    }
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        builder.build_float_add(lhs, rhs, "").unwrap().into()
                    }
                    (_, _) => {
                        panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or FloatValue")
                    }
                }
            }

            _ => todo!("Unimplemented lowering for {self}"),
        })
    }
}
