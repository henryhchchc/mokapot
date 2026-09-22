//! IR fixtures for unit tests, importable with `use crate::ir::test::prelude::*;`.
//!
//! The module name ends in `prelude` so glob imports do not trip
//! `clippy::wildcard_imports`.

use std::collections::{HashMap, HashSet, VecDeque};

pub(crate) use crate::ir::{
    BasicBlock, BlockId, BlockKind, BlockParameter, ControlTransfer, InstructionLocation,
    MethodEntry, MokaIRMethod, Operation, Successor, Terminator, ValueId, expression::Expression,
};
use crate::{
    ir::NumericalId,
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, ProgramCounter},
        method::AccessFlags,
        references::ClassRef,
    },
    types::reference_type::ReferenceType,
};

/// Asserts that `value` matches `pattern`, printing the value on failure.
///
/// `std`'s `assert_matches!` is not stable yet.
macro_rules! assert_matches {
    ($value:expr, $($pattern:tt)+) => {
        match $value {
            $($pattern)+ => {}
            ref value => panic!(
                "assertion failed: `{:?}` does not match `{}`",
                value,
                stringify!($($pattern)+),
            ),
        }
    };
}
pub(crate) use assert_matches;

/// `N` consecutive identities starting at `base`.
///
/// The kind is inferred from use: `let [a, b] = ids(0);` binds either `ValueId`s or `BlockId`s
/// depending on how `a` and `b` are used.
pub(crate) fn ids<const N: usize, T: NumericalId>(base: u32) -> [T; N] {
    std::array::from_fn(|offset| {
        T::from_raw(base + u32::try_from(offset).expect("a test declares few ids"))
    })
}

/// Builds a `static` method to lift from `instructions`.
pub(crate) fn method<I, PC>(
    instructions: I,
    descriptor: &str,
    exception_table: Vec<ExceptionTableEntry>,
) -> Method
where
    I: IntoIterator<Item = (PC, Instruction)>,
    PC: Into<ProgramCounter>,
{
    crate::tests::method(
        instructions,
        descriptor,
        exception_table,
        AccessFlags::PUBLIC | AccessFlags::STATIC,
    )
}

/// The class named `name`.
pub(crate) fn cls_r(name: &str) -> ClassRef {
    name.parse().expect("a valid class name")
}

/// The reference type named `name`.
pub(crate) fn ref_t(name: &str) -> ReferenceType {
    name.parse().expect("a valid type name")
}

/// The method entry invoking `target` with `args`.
pub(crate) fn method_entry(
    target: BlockId,
    args: impl IntoIterator<Item = ValueId>,
) -> MethodEntry {
    MethodEntry {
        target,
        arguments: args.into_iter().collect(),
    }
}

/// A `Code` block keyed by `id`, with `params` and `ops`.
pub(crate) fn bb(
    id: BlockId,
    params: impl IntoIterator<Item = ValueId>,
    ops: &[Operation],
    term: Terminator,
) -> (BlockId, BasicBlock) {
    keyed(id, BlockKind::Code, params, ops, term)
}

/// A parameterless, operationless `Code` block keyed by `id`.
pub(crate) fn code(id: BlockId, term: Terminator) -> (BlockId, BasicBlock) {
    bb(id, [], &[], term)
}

/// A landing pad keyed by `id`, defining `exception` on entry.
pub(crate) fn landing_pad(
    id: BlockId,
    exception: ValueId,
    params: impl IntoIterator<Item = ValueId>,
    ops: &[Operation],
    term: Terminator,
) -> (BlockId, BasicBlock) {
    keyed(id, BlockKind::LandingPad { exception }, params, ops, term)
}

fn keyed(
    id: BlockId,
    kind: BlockKind,
    params: impl IntoIterator<Item = ValueId>,
    ops: &[Operation],
    term: Terminator,
) -> (BlockId, BasicBlock) {
    let block = BasicBlock {
        kind,
        parameters: params
            .into_iter()
            .map(|value| BlockParameter { value })
            .collect(),
        operations: ops.to_vec(),
        terminator: term,
    };
    (id, block)
}

/// A `Block` arm to `target` carrying `args` under `transfer`.
pub(crate) fn arm<I>(target: BlockId, args: I, transfer: ControlTransfer) -> Successor
where
    I: IntoIterator<Item = ValueId>,
{
    Successor::Block {
        target,
        arguments: args.into_iter().collect(),
        transfer,
    }
}

/// An unconditional `Block` arm to `target`.
pub(crate) fn edge(target: BlockId, args: impl IntoIterator<Item = ValueId>) -> Successor {
    arm(target, args, ControlTransfer::Unconditional)
}

/// A `goto` to `target`.
pub(crate) fn goto(target: BlockId, args: impl IntoIterator<Item = ValueId>) -> Terminator {
    goto_with(target, args, ControlTransfer::Unconditional)
}

/// A `goto` to `target` carrying `transfer`.
pub(crate) fn goto_with(
    target: BlockId,
    args: impl IntoIterator<Item = ValueId>,
    transfer: ControlTransfer,
) -> Terminator {
    Terminator::Goto {
        target: arm(target, args, transfer),
    }
}

/// A two-way `branch` selecting between `taken` and `otherwise`.
pub(crate) fn branch(taken: Successor, otherwise: Successor) -> Terminator {
    Terminator::Branch { taken, otherwise }
}

/// A `try` of `op` with `normal` and `exceptional` outcomes.
pub(crate) fn try_op(op: Operation, normal: Successor, exceptional: Vec<Successor>) -> Terminator {
    Terminator::Try {
        operation: op,
        normal,
        exceptional,
    }
}

/// A `return` of `value`.
pub(crate) fn ret(value: ValueId) -> Terminator {
    Terminator::Return { value: Some(value) }
}

/// A `void` return.
pub(crate) fn void() -> Terminator {
    Terminator::Return { value: None }
}

/// An operation evaluated only for its effects.
pub(crate) fn effect(expr: impl Into<Expression>) -> Operation {
    Operation::Effect { expr: expr.into() }
}

/// An operation defining `value` from `expr`.
pub(crate) fn def(value: ValueId, expr: impl Into<Expression>) -> Operation {
    Operation::Definition {
        value,
        expr: expr.into(),
    }
}

/// The blocks reachable from the entry of `ir`.
pub(crate) fn reachable_blocks(ir: &MokaIRMethod) -> Vec<(BlockId, &BasicBlock)> {
    reachable_of(&ir.blocks, ir.entry_block())
}

/// The blocks reachable from `entry` within `blocks`.
pub(crate) fn reachable_of(
    blocks: &HashMap<BlockId, BasicBlock>,
    entry: BlockId,
) -> Vec<(BlockId, &BasicBlock)> {
    let mut result = Vec::new();
    let mut visited = HashSet::new();
    let mut pending = VecDeque::from([entry]);
    while let Some(id) = pending.pop_front() {
        if !visited.insert(id) {
            continue;
        }
        let block = blocks
            .get(&id)
            .expect("a successor must belong to its method");
        pending.extend(
            block
                .terminator
                .successors()
                .filter_map(Successor::block_target),
        );
        result.push((id, block));
    }
    result
}

/// The operations of `ir`'s reachable blocks.
pub(crate) fn operations(ir: &MokaIRMethod) -> impl Iterator<Item = &Operation> {
    reachable_blocks(ir)
        .into_iter()
        .flat_map(|(_, it)| &it.operations)
}

/// The operations of `ir`'s reachable terminators.
pub(crate) fn terminator_operations(ir: &MokaIRMethod) -> impl Iterator<Item = &Operation> {
    reachable_blocks(ir)
        .into_iter()
        .filter_map(|(_, it)| it.terminator.operation())
}

/// The JVM origin of `ir`'s entry terminator.
pub(crate) fn entry_origin(ir: &MokaIRMethod) -> Option<ProgramCounter> {
    ir.source_map().origin_of(InstructionLocation::Terminator {
        block: ir.entry_block(),
    })
}
