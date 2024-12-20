//! Backend for generating LLVM IR for usage with tools provided by the LLVM
//! infrastructure.

use inkwell::{
    basic_block::BasicBlock, builder::Builder, module::Module, values::BasicValueEnum, IntPredicate,
};

use crate::{
    ir::{expression::Condition, MokaInstruction, Operand},
    jvm::code::ProgramCounter,
};
use utils::get_or_insert_basic_block_ordered;

mod intrinsics;
mod utils;

/// Trait representing a struct that can be lowered into LLVM IR.
pub trait IRLowering {
    /// Lowers the LLVM IR representation of this struct and inserts it into the
    /// [`Module`].
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>>;
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
                    let Some(lhs) = lhs.lower(module, builder, pc) else {
                        panic!("Expect LHS operand {lhs:?} to lower to a BasicValue")
                    };
                    let Some(rhs) = rhs.lower(module, builder, pc) else {
                        panic!("Expect LHS operand {rhs:?} to lower to a BasicValue")
                    };

                    match (lhs, rhs) {
                        (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => builder
                            .build_int_compare(IntPredicate::EQ, lhs, rhs, "")
                            .unwrap(),

                        (BasicValueEnum::PointerValue(lhs), BasicValueEnum::PointerValue(rhs)) => {
                            let ctx = module.get_context();
                            
                            // TODO(Derppening): Should we be assuming 32-bit pointer types?
                            let lhs = builder.build_ptr_to_int(lhs, ctx.i32_type(), "").unwrap();
                            let rhs = builder.build_ptr_to_int(rhs, ctx.i32_type(), "").unwrap();

                            builder
                                .build_int_compare(IntPredicate::EQ, lhs, rhs, "")
                                .unwrap()
                        }

                        (_, _) => {
                            panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or PointerValue")
                        }
                    }
                }

                Condition::NotEqual(lhs, rhs) => {
                    let Some(lhs) = lhs.lower(module, builder, pc) else {
                        panic!("Expect LHS operand {lhs:?} to lower to a BasicValue")
                    };
                    let Some(rhs) = rhs.lower(module, builder, pc) else {
                        panic!("Expect LHS operand {rhs:?} to lower to a BasicValue")
                    };

                    match (lhs, rhs) {
                        (BasicValueEnum::IntValue(lhs), BasicValueEnum::IntValue(rhs)) => builder
                            .build_int_compare(IntPredicate::NE, lhs, rhs, "")
                            .unwrap(),

                        (BasicValueEnum::PointerValue(lhs), BasicValueEnum::PointerValue(rhs)) => {
                            let ctx = module.get_context();

                            // TODO(Derppening): Should we be assuming 32-bit pointer types?
                            let lhs = builder.build_ptr_to_int(lhs, ctx.i32_type(), "").unwrap();
                            let rhs = builder.build_ptr_to_int(rhs, ctx.i32_type(), "").unwrap();

                            builder
                                .build_int_compare(IntPredicate::NE, lhs, rhs, "")
                                .unwrap()
                        }

                        (_, _) => {
                            panic!("Expect ({lhs:?}, {rhs:?}) to both be IntValue or PointerValue")
                        }
                    }
                }

                Condition::IsNull(operand) => {
                    let Some(BasicValueEnum::PointerValue(operand)) =
                        operand.lower(module, builder, pc)
                    else {
                        panic!("Expect {operand:?} to lower to a PointerValue")
                    };

                    builder.build_is_null(operand, "").unwrap()
                }

                Condition::IsNotNull(operand) => {
                    let Some(BasicValueEnum::PointerValue(operand)) =
                        operand.lower(module, builder, pc)
                    else {
                        panic!("Expect {operand:?} to lower to a PointerValue")
                    };

                    builder.build_is_not_null(operand, "").unwrap()
                }

                Condition::IsZero(operand) => {
                    let Some(BasicValueEnum::IntValue(operand)) =
                        operand.lower(module, builder, pc)
                    else {
                        panic!("Expect {operand:?} to lower to an IntValue")
                    };

                    builder
                        .build_int_compare(
                            IntPredicate::EQ,
                            operand,
                            operand.get_type().const_zero(),
                            "",
                        )
                        .unwrap()
                }

                Condition::IsNonZero(operand) => {
                    let Some(BasicValueEnum::IntValue(operand)) =
                        operand.lower(module, builder, pc)
                    else {
                        panic!("Expect {operand:?} to lower to an IntValue")
                    };

                    builder
                        .build_int_compare(
                            IntPredicate::NE,
                            operand,
                            operand.get_type().const_zero(),
                            "",
                        )
                        .unwrap()
                }

                _ => todo!("Unimplemented lowering for {self}"),
            }
            .into(),
        )
    }
}

impl IRLowering for Operand {
    fn lower<'ctx>(
        &self,
        module: &Module<'ctx>,
        builder: &Builder<'ctx>,
        pc: ProgramCounter,
    ) -> Option<BasicValueEnum<'ctx>> {
        todo!("Unimplemented lowering for {self}")
    }
}
