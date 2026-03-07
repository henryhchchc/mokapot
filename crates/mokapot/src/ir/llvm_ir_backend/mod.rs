//! Backend for generating LLVM IR for usage with tools provided by the LLVM
//! infrastructure.

use std::collections::HashMap;

use inkwell::{
    AddressSpace, IntPredicate,
    basic_block::BasicBlock,
    builder::Builder,
    context::ContextRef,
    module::Module,
    values::{BasicValueEnum, IntValue, PointerValue},
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

struct Context<'ctx, 'a> {
    ctx: ContextRef<'ctx>,
    module: &'a Module<'ctx>,
    builder: &'a Builder<'ctx>,
    _vars: HashMap<Identifier, PointerValue<'ctx>>,
}

impl<'ctx, 'a> Context<'ctx, 'a> {
    fn new(ctx: ContextRef<'ctx>, module: &'a Module<'ctx>, builder: &'a Builder<'ctx>) -> Self {
        Self {
            ctx,
            module,
            builder,
            _vars: HashMap::new(),
        }
    }
}

/// Trait representing a struct that can be lowered into LLVM IR.
// TODO(Derppening): Determine if this trait can be used for all lower-able constructs.
trait IRLowering {
    /// The type produced by the lowering operation.
    type Output<'ctx>;

    /// Lowers the LLVM IR representation of this struct and inserts it into the
    /// [`Module`].
    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, pc: ProgramCounter) -> Self::Output<'ctx>;

    /// Lowers an equality operation and inserts it into the [`Builder`].
    fn lower_eq_op<'ctx>(
        &self,
        ctx: &mut Context<'ctx, '_>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        negated: bool,
    ) -> IntValue<'ctx> {
        let lhs = lhs.lower(ctx, pc);
        let rhs = rhs.lower(ctx, pc);

        let predicate = if negated {
            IntPredicate::NE
        } else {
            IntPredicate::EQ
        };

        match (lhs, rhs) {
            (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => ctx
                .builder
                .build_int_compare(predicate, lhs, rhs, "")
                .unwrap(),

            (BasicValueEnum::PointerValue(lhs), BasicValueEnum::PointerValue(rhs)) => {
                // TODO(Derppening): Should we be assuming 32-bit pointer types?
                let lhs = ctx
                    .builder
                    .build_ptr_to_int(lhs, ctx.ctx.i32_type(), "")
                    .unwrap();
                let rhs = ctx
                    .builder
                    .build_ptr_to_int(rhs, ctx.ctx.i32_type(), "")
                    .unwrap();

                ctx.builder
                    .build_int_compare(predicate, lhs, rhs, "")
                    .unwrap()
            }

            (_, _) => {
                panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or PointerValue")
            }
        }
    }

    /// Lowers an integer comparison operation and inserts it into the [`Builder`].
    fn lower_cmp_op<'ctx>(
        &self,
        ctx: &mut Context<'ctx, '_>,
        pc: ProgramCounter,
        lhs: &Operand,
        rhs: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> IntValue<'ctx> {
        let BasicValueEnum::IntValue(lhs) = lhs.lower(ctx, pc) else {
            panic!("Expect LHS operand {lhs:?} to lower to an IntValue")
        };
        let BasicValueEnum::IntValue(rhs) = rhs.lower(ctx, pc) else {
            panic!("Expect RHS operand {rhs:?} to lower to a BasicValue")
        };

        ctx.builder
            .build_int_compare(llvm_cmpop, lhs, rhs, "")
            .unwrap()
    }

    /// Lowers a null check and inserts it into the [`Builder`].
    fn lower_null_check<'ctx>(
        &self,
        ctx: &mut Context<'ctx, '_>,
        pc: ProgramCounter,
        operand: &Operand,
        negated: bool,
    ) -> IntValue<'ctx> {
        let BasicValueEnum::PointerValue(operand) = operand.lower(ctx, pc) else {
            panic!("Expect {operand:?} to lower to a PointerValue")
        };

        if negated {
            ctx.builder.build_is_not_null(operand, "").unwrap()
        } else {
            ctx.builder.build_is_null(operand, "").unwrap()
        }
    }

    /// Lowers a compare-with-zero operation and inserts it into the [`Builder`].
    ///
    /// Effectively inserts `{llvm_cmpop} {operand}, 0`.
    fn lower_cmp_zero_op<'ctx>(
        &self,
        ctx: &mut Context<'ctx, '_>,
        pc: ProgramCounter,
        operand: &Operand,
        llvm_cmpop: IntPredicate,
    ) -> IntValue<'ctx> {
        let BasicValueEnum::IntValue(operand) = operand.lower(ctx, pc) else {
            panic!("Expect {operand:?} to lower to an IntValue")
        };

        ctx.builder
            .build_int_compare(llvm_cmpop, operand, operand.get_type().const_zero(), "")
            .unwrap()
    }
}

impl IRLowering for MokaInstruction {
    type Output<'ctx> = ();

    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, pc: ProgramCounter) -> Self::Output<'ctx> {
        let func_val = ctx
            .builder
            .get_insert_block()
            .and_then(BasicBlock::get_parent)
            .unwrap();
        let this_bb = get_or_insert_basic_block_ordered(ctx, func_val, pc);

        // If the previous BB is not terminated, add a jmp to this BB
        if ctx
            .builder
            .get_insert_block()
            .map(BasicBlock::get_terminator)
            .is_none()
        {
            ctx.builder.build_unconditional_branch(this_bb).unwrap();
        }

        ctx.builder.position_at_end(this_bb);

        match self {
            MokaInstruction::Nop => intrinsics::invoke_donothing(ctx),
            MokaInstruction::Jump { condition, target } => {
                let target_bb = get_or_insert_basic_block_ordered(ctx, func_val, *target);

                if let Some(condition) = condition {
                    let condition = condition.lower(ctx, pc);

                    let current_bb = ctx.builder.get_insert_block().unwrap();
                    let context = current_bb.get_context();

                    let then_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.then"));
                    let else_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.else"));
                    let cont_bb =
                        context.insert_basic_block_after(current_bb, &format!("{target}.cont"));

                    ctx.builder
                        .build_conditional_branch(condition, then_bb, else_bb)
                        .unwrap();

                    ctx.builder.position_at_end(then_bb);
                    ctx.builder.build_unconditional_branch(target_bb).unwrap();

                    ctx.builder.position_at_end(else_bb);
                    ctx.builder.build_unconditional_branch(cont_bb).unwrap();

                    ctx.builder.position_at_end(cont_bb);
                } else {
                    ctx.builder.build_unconditional_branch(target_bb).unwrap();
                }
            }

            MokaInstruction::Return(operand) => {
                if let Some(operand) = operand {
                    let operand = operand.lower(ctx, pc);

                    ctx.builder.build_return(Some(&operand)).unwrap();
                } else {
                    ctx.builder.build_return(None).unwrap();
                }
            }

            _ => todo!("Unimplemented lowering for {self}"),
        }
    }
}

impl IRLowering for Condition {
    type Output<'ctx> = IntValue<'ctx>;

    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, pc: ProgramCounter) -> Self::Output<'ctx> {
        match self {
            Condition::Equal(lhs, rhs) => self.lower_eq_op(ctx, pc, lhs, rhs, false),
            Condition::NotEqual(lhs, rhs) => self.lower_eq_op(ctx, pc, lhs, rhs, true),

            Condition::LessThan(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SLT)
            }
            Condition::LessThanOrEqual(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SLE)
            }
            Condition::GreaterThan(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SGT)
            }
            Condition::GreaterThanOrEqual(lhs, rhs) => {
                self.lower_cmp_op(ctx, pc, lhs, rhs, IntPredicate::SGE)
            }

            Condition::IsNull(operand) => self.lower_null_check(ctx, pc, operand, false),
            Condition::IsNotNull(operand) => self.lower_null_check(ctx, pc, operand, true),

            Condition::IsZero(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::EQ)
            }
            Condition::IsNonZero(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::NE)
            }
            Condition::IsPositive(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SGT)
            }
            Condition::IsNegative(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SLT)
            }
            Condition::IsNonNegative(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SGE)
            }
            Condition::IsNonPositive(operand) => {
                self.lower_cmp_zero_op(ctx, pc, operand, IntPredicate::SLE)
            }
        }
    }
}

impl IRLowering for ConstantValue {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, _: ProgramCounter) -> Self::Output<'ctx> {
        match self {
            ConstantValue::Null => ctx
                .ctx
                .ptr_type(AddressSpace::default())
                .const_null()
                .into(),
            ConstantValue::Integer(v) => {
                ctx.ctx.i32_type().const_int(upcast_to_u64(*v), true).into()
            }
            ConstantValue::Float(v) => ctx.ctx.f32_type().const_float(f64::from(*v)).into(),
            ConstantValue::Long(v) => ctx.ctx.i64_type().const_int(upcast_to_u64(*v), true).into(),
            ConstantValue::Double(v) => ctx.ctx.f64_type().const_float(*v).into(),

            _ => todo!("Unimplemented lowering for {self}"),
        }
    }
}

impl IRLowering for Expression {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, pc: ProgramCounter) -> Self::Output<'ctx> {
        match self {
            Expression::Const(value) => value.lower(ctx, pc),
            Expression::Math(op) => op.lower(ctx, pc),

            _ => todo!("Unimplemented lowering for {self}"),
        }
    }
}

impl IRLowering for Identifier {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(&self, _ctx: &mut Context<'ctx, '_>, _pc: ProgramCounter) -> Self::Output<'ctx> {
        todo!("Unimplemented lowering for {self}")
    }
}

impl IRLowering for Operand {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(&self, _ctx: &mut Context<'ctx, '_>, _pc: ProgramCounter) -> Self::Output<'ctx> {
        todo!("Unimplemented lowering for {self}")
    }
}

impl IRLowering for MathOperation {
    type Output<'ctx> = BasicValueEnum<'ctx>;

    fn lower<'ctx>(&self, ctx: &mut Context<'ctx, '_>, pc: ProgramCounter) -> Self::Output<'ctx> {
        match self {
            MathOperation::Add(lhs, rhs) => {
                let lhs = lhs.lower(ctx, pc);
                let rhs = rhs.lower(ctx, pc);

                match (lhs, rhs) {
                    (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => {
                        ctx.builder.build_int_add(lhs, rhs, "").unwrap().into()
                    }
                    (BasicValueEnum::FloatValue(lhs), BasicValueEnum::FloatValue(rhs)) => {
                        ctx.builder.build_float_add(lhs, rhs, "").unwrap().into()
                    }
                    (_, _) => {
                        panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or FloatValue")
                    }
                }
            }

            _ => todo!("Unimplemented lowering for {self}"),
        }
    }
}
