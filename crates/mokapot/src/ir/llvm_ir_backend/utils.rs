use std::str::FromStr;

use inkwell::{basic_block::BasicBlock, context::ContextRef, values::FunctionValue};
use num_traits::{AsPrimitive, NumCast, PrimInt};

use crate::jvm::code::ProgramCounter;

/// Sign-extends an integer number into [`u64`] for consumption by the LLVM API.
pub(super) fn upcast_to_u64<T: PrimInt>(value: T) -> u64 {
    assert!(size_of::<T>() * 8 <= u64::BITS as usize);

    if value >= T::zero() {
        <u64 as NumCast>::from(value).unwrap()
    } else {
        <i64 as NumCast>::from(value).map(AsPrimitive::as_).unwrap()
    }
}

/// Retrieves an existing [`BasicBlock`] in the [`function_value`][FunctionValue], or inserts a new
/// basic block in `function_value`, preserving the order of IR instructions using the provided
/// [`pc`][ProgramCounter].
pub(super) fn get_or_insert_basic_block_ordered<'ctx>(
    context: ContextRef<'ctx>,
    function_value: FunctionValue<'ctx>,
    pc: ProgramCounter,
) -> BasicBlock<'ctx> {
    let bb = function_value
        .get_basic_block_iter()
        .find(|bb| bb.get_name().to_str().unwrap() == pc.to_string());

    if let Some(bb) = bb {
        return bb;
    }

    let pc_bb_name = pc.to_string();

    // Find the last BB before where we should be inserted
    let insertion_point = function_value
        .get_basic_block_iter()
        .filter(|bb| u16::from_str(bb.get_name().to_str().unwrap()).is_ok())
        .filter(|bb| {
            // TODO(Derppening): Hax - We should use a lookup table for this
            ProgramCounter::from(u16::from_str(bb.get_name().to_str().unwrap()).unwrap()) < pc
        })
        .last();

    if let Some(insert_bb) = insertion_point {
        // Found a BB to insert after - Insert after the insertion point
        let ctx = insert_bb.get_context();

        ctx.insert_basic_block_after(insert_bb, &pc_bb_name)
    } else if let Some(insert_bb) = function_value.get_first_basic_block() {
        // No BB to append after but function already contains BBs - Prepend to function start
        let ctx = insert_bb.get_context();

        ctx.prepend_basic_block(insert_bb, &pc_bb_name)
    } else {
        // Function doesn't have any BBs - Create the first BB
        context.append_basic_block(function_value, &pc_bb_name)
    }
}
