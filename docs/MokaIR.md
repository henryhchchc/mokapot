# MokaIR

MokaIR is mokapot's register-based, scalar static single-assignment (SSA)
representation of a JVM method. It exposes semantic operations and control
flow without exposing the JVM operand stack or local-variable slots.

MokaIR is currently unstable. Enable the `unstable-moka-ir` feature (or the
umbrella `unstable` feature) to use it.

## Constructing a method

Parse a class file, select a method with a body, and call
[`MokaIRMethod::from_method`](https://docs.rs/mokapot/latest/mokapot/ir/struct.MokaIRMethod.html#method.from_method):

```rust,no_run
use std::{fs::File, io::BufReader};

use mokapot::{ir::MokaIRMethod, jvm::Class};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut reader = BufReader::new(File::open("Example.class")?);
    let class = Class::from_reader(&mut reader)?;

    for method in &class.methods {
        if method.body.is_none() {
            continue;
        }

        let ir = MokaIRMethod::from_method(method)?;
        println!("{} has {} reachable blocks", ir.name(), ir.blocks().len());
    }

    Ok(())
}
```

Construction includes only blocks reachable from method entry. A method
without bytecode, or reachable bytecode that cannot be represented as valid
MokaIR, returns `MokaIRBuildError`; construction does not expose a partial
method.

## Blocks and instructions

`MokaIRMethod::blocks` returns maximal basic blocks in deterministic source
order. The method's `entry_block` identifies where execution begins. Each
block contains, in order:

1. zero or more block-entry `Phi` nodes;
2. zero or more semantic `Operation`s;
3. exactly one `Terminator`.

The terminator's ordered `Successor` arms are the authoritative control-flow
edges. Each arm has a target block and a `ControlTransfer`: unconditional,
conditional, normal, exception-table, or unwind. Separate arms remain
separate even when they have the same source and target.

```rust,no_run
# use mokapot::ir::MokaIRMethod;
# use mokapot::jvm::Method;
fn inspect(method: &Method) -> Result<(), mokapot::ir::MokaIRBuildError> {
    let ir = MokaIRMethod::from_method(method)?;

    for block in ir.blocks() {
        println!("{}:", block.id());
        for phi in block.phis() {
            println!("  {} defines {}", phi.id(), phi.value());
        }
        for operation in block.operations() {
            println!("  {}: {}", operation.id(), operation);
        }
        println!("  {}: {}", block.terminator().id(), block.terminator());

        for successor in block.terminator().successors() {
            println!(
                "    {} -> {} ({:?})",
                successor.id(),
                successor.target(),
                successor.transfer(),
            );
        }
    }

    Ok(())
}
```

`BlockId`, `InstructionId`, `EdgeId`, and `ValueId` are opaque, method-local
identities. Do not derive relationships from their displayed numbers or reuse
an identity with another method.

## Scalar SSA values

Every operand names one `ValueId`, and every value has exactly one
`ValueDefinition`. Definitions include the receiver (`this`), parameters,
caught exceptions, phi nodes, and value-producing operations. Use
`MokaIRMethod::definition_of` to locate a value's definition.

`OperationKind::Definition` evaluates an expression and defines a value.
`OperationKind::Effect` evaluates an expression only for its effects. For
example, a field or array write and a void call remain ordered operations but
do not receive meaningless result values.

A phi input associates a value with the predecessor block that selects it:

```rust,no_run
# use mokapot::ir::MokaIRMethod;
# use mokapot::jvm::Method;
# fn inspect(method: &Method) -> Result<(), mokapot::ir::MokaIRBuildError> {
# let ir = MokaIRMethod::from_method(method)?;
for block in ir.blocks() {
    for phi in block.phis() {
        for input in phi.inputs() {
            println!(
                "{} receives {} from {}",
                phi.value(),
                input.value(),
                input.predecessor(),
            );
        }
    }
}
# Ok(())
# }
```

Loop-carried values can make this graph cyclic. Trivial phis are removed during
construction. JVM loads, stores, stack manipulation, and `nop` affect lifting
state but do not produce placeholder MokaIR instructions.

`DefUseChain` builds an owned method-local definition/use index. Phi uses retain
the predecessor that selects them:

```rust,no_run
# use mokapot::ir::{DefUseChain, MokaIRMethod};
# use mokapot::jvm::Method;
# fn inspect(method: &Method) -> Result<(), mokapot::ir::MokaIRBuildError> {
# let ir = MokaIRMethod::from_method(method)?;
let def_use = DefUseChain::new(&ir);
for value in ir.parameter_values() {
    println!("{value}: {:?}", def_use.definition_of(*value));
    for use_site in def_use.uses_of(*value) {
        println!("  used at {use_site:?}");
    }
}
# Ok(())
# }
```

## Exceptional and legacy control flow

A potentially throwing definition or effect with modeled handlers ends its
block with normal and exceptional arms. A value-producing operation's result
is available only on its normal successor; exceptional successors receive
pre-operation locals. A throw, or a return whose method exit can fail, can have
only exceptional arms. Exception-table arms preserve JVM order and are followed
by an unwind arm when no modeled handler is exhaustive.

Each reachable handler context starts with a synthetic handler-entry block and
a distinct caught-exception value. Query it with
`MokaIRMethod::caught_exception`. Synthetic handler entries and unwind nodes do
not claim source provenance.

Legacy `jsr` and `ret` subroutines are expanded context-sensitively during
lifting. Completed MokaIR contains only ordinary control flow; one JVM program
counter can therefore correspond to several IR nodes.

## Source provenance and coverage

`MokaIRMethod::source_map` returns a sparse, bidirectional relation rather than
a bytecode-to-IR bijection:

- `instructions_at(pc)` returns every directly related MokaIR instruction;
- `origins_of(id)` returns every directly related JVM program counter.

Either iterator may be empty. Erased JVM stack/local operations can have no IR
node, while phis, synthetic handler entries, preheaders, and unwind nodes can
have no JVM origin. A normalized legacy instruction can have multiple related
IR nodes.

For coverage transfer, mark the nodes returned by `instructions_at` for each
covered JVM program counter. Do not infer coverage for a node whose
`origins_of` iterator is empty.

## Analysis views

`MokaIRMethod::control_flow_graph` derives a borrowed graph solely from block
terminators. It supports node and edge iteration, outgoing edges, exit blocks,
and path-condition analysis. The optional `petgraph` feature adds petgraph CFG
traits without introducing a second source of control-flow truth.

Together, the block CFG, `DefUseChain`, and sparse `SourceMap` provide the
public views needed for control-flow, scalar data-flow, and JVM-origin-aware
analysis.
