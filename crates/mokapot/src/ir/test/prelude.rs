//! IR fixtures for unit tests, importable with `use crate::ir::test::prelude::*;`.

use crate::types::class_name::ClassName;

use std::collections::{BTreeSet, HashMap, HashSet, VecDeque};

pub(crate) use crate::ir::{
    BasicBlock, BlockId, BlockKind, BlockParameter, ControlTransfer, InstructionLocation,
    MethodEntry, MokaIRMethod, Operation, SourceMap, Successor, Terminator, ValueId,
    expression::{Expression, InvocationKind},
};
use crate::{
    analysis::fixed_point::JoinSemiLattice,
    ir::NumericalId,
    jvm::{
        Method,
        code::{ExceptionTableEntry, Instruction, ProgramCounter},
        method::AccessFlags,
        references::MethodRef,
    },
    types::reference_type::ReferenceType,
};

/// A set-valued fact used by fixed-point analysis tests.
#[derive(Debug, Clone, PartialEq, Eq, Default, proptest_derive::Arbitrary)]
pub(crate) struct TestSet(pub(crate) BTreeSet<u8>);

impl<I> From<I> for TestSet
where
    I: IntoIterator<Item = u8>,
{
    fn from(value: I) -> Self {
        Self(value.into_iter().collect())
    }
}

impl PartialOrd for TestSet {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        if self == other {
            Some(std::cmp::Ordering::Equal)
        } else if self.0.is_subset(&other.0) {
            Some(std::cmp::Ordering::Less)
        } else if self.0.is_superset(&other.0) {
            Some(std::cmp::Ordering::Greater)
        } else {
            None
        }
    }
}

impl JoinSemiLattice for TestSet {
    fn join_assign(&mut self, other: Self) -> bool {
        let old_len = self.0.len();
        self.0.extend(other.0);
        self.0.len() != old_len
    }
}

/// Asserts that `value` matches `pattern`, printing the value on failure.
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

/// `N` consecutive identities starting at `base`, with the id kind inferred from use.
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
pub(crate) fn cls_name(name: &str) -> ClassName {
    name.parse().expect("a valid class name")
}

/// The reference type named `name`.
pub(crate) fn ref_t(name: &str) -> ReferenceType {
    name.parse().expect("a valid type name")
}

/// A symbolic method on `java/lang/Object` with the given name and descriptor.
pub(crate) fn method_ref(name: &str, descriptor: &str) -> MethodRef {
    MethodRef {
        owner: ref_t("java/lang/Object"),
        name: name.to_owned(),
        descriptor: descriptor.parse().expect("a valid method descriptor"),
    }
}

/// A call with the given dispatch kind, symbolic method, and arguments.
pub(crate) fn call(
    kind: InvocationKind,
    method: MethodRef,
    args: impl IntoIterator<Item = ValueId>,
) -> Expression {
    Expression::Call {
        kind,
        method,
        args: args.into_iter().collect(),
    }
}

/// Fresh receiver and parameter identities for method-entry tests.
pub(crate) fn entry_values(
    instance: bool,
    parameter_count: usize,
) -> (Option<ValueId>, Vec<ValueId>) {
    let mut allocator = crate::ir::IdAllocator::default();
    let receiver = instance.then(|| allocator.new_id());
    let parameters = (0..parameter_count).map(|_| allocator.new_id()).collect();
    (receiver, parameters)
}

/// The method entry invoking `target` with `args`.
pub(crate) fn method_entry(
    target: BlockId,
    args: impl IntoIterator<Item = ValueId>,
) -> MethodEntry {
    MethodEntry {
        block: target,
        arguments: args.into_iter().collect(),
    }
}

/// A `MokaIRMethod` over `blocks` entering at `entry`.
pub(crate) fn ir_method(entry: BlockId, blocks: HashMap<BlockId, BasicBlock>) -> MokaIRMethod {
    MokaIRMethod {
        access_flags: AccessFlags::PUBLIC | AccessFlags::STATIC,
        name: "test".to_owned(),
        descriptor: "()V".parse().expect("a valid descriptor"),
        owner: cls_name("java/lang/Object"),
        entry: method_entry(entry, []),
        blocks,
        source_map: SourceMap::new(),
        this: None,
        parameters: Vec::new(),
    }
}

/// A `Code` block keyed by `id`, with `params` and `ops`.
pub(crate) fn bb(
    id: BlockId,
    params: impl IntoIterator<Item = ValueId>,
    ops: &[Operation],
    term: Terminator,
) -> (BlockId, BasicBlock) {
    let block = BasicBlock {
        kind: BlockKind::Code,
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

/// A parameterless, operationless `Code` block keyed by `id`.
pub(crate) fn code(id: BlockId, term: Terminator) -> (BlockId, BasicBlock) {
    bb(id, [], &[], term)
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

/// A `return` of `value`, which may unwind.
pub(crate) fn ret(value: ValueId) -> Terminator {
    Terminator::Return {
        value: Some(value),
        exceptional: vec![Successor::Unwind],
    }
}

/// A `void` return, which may unwind.
pub(crate) fn void() -> Terminator {
    Terminator::Return {
        value: None,
        exceptional: vec![Successor::Unwind],
    }
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
    reachable_of(&ir.blocks, ir.entry.block)
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
    ir.source_map.origin_of(InstructionLocation::Terminator {
        block: ir.entry.block,
    })
}
