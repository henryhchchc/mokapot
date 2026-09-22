# MokaIR

MokaIR is mokapot's register-based, scalar static single-assignment (SSA) representation of a JVM method.
It exposes semantic operations and control flow without exposing the JVM operand stack or local-variable slots.
It is unstable: enable `unstable-moka-ir` (or the umbrella `unstable` feature).

## Construction

`MokaIRMethod::from_method` lifts a method with a body, emitting only blocks reachable from entry.
Malformed bytecode, unsupported bytecode, or invalid frame state returns `MokaIRBuildError`; there is no partial method.

## Blocks

A method is a set of maximal basic blocks addressed by opaque `BlockId`s, with no ordering; `entry_block` and `entry` locate execution start and the arguments it receives.
Each block carries a `kind` (`Code`, or a `LandingPad` that defines the caught exception), entry-bound `parameters`, ordered `operations`, and exactly one `terminator`.

A terminator's ordered `Successor` arms are the authoritative control-flow edges.
An arm is a block edge carrying a target, arguments, and a `ControlTransfer` — unconditional, conditional (`BranchGuard`), or exceptional (`None` catch type is catch-all) — or an unwind exit.
Parallel arms stay distinct, preserving switch cases and handler precedence.

## Values

Every operand is one `ValueId` with exactly one `ValueDefinition`: `This`, a `Parameter`, a landing-pad `CaughtException`, or an `Instruction`, findable with `definition_of`.
`Operation::Definition` produces a value; `Operation::Effect` (a write, or a void call) only orders effects.

A `BlockParameter` is bound on entry from the arguments of each incoming arm, which line up with the target's parameters; `entry` supplies the entry block's arguments the same way.
Loop-carried arguments make the graph cyclic, and a parameter that only forwards one value is eliminated, so parameters mark genuine joins.

Instructions are addressed structurally, not by identity: an `InstructionLocation` (block parameter, operation, or terminator) resolved through `instruction`.
`BlockId` and `ValueId` are opaque and method-local.

## Exceptions

A fallible definition or effect ends its block with a `Try` terminator: an unguarded normal arm first, then exceptional arms in exception-table order, with a trailing unwind arm when no handler catches all.
The operation's value is available only on the normal arm; exceptional arms see pre-operation locals.
Every return may unwind, so a returned value ends its block with a `Return` whose arms are all exceptional.
`Throw` likewise has only exceptional arms.
Each handler context begins with a landing-pad block defining its caught exception.
Legacy `jsr`/`ret` subroutines are unmodeled: any occurrence, reachable or not, is rejected.

## Source provenance

`source_map` exposes a sparse, bidirectional relation, not a bijection: `instructions_at(pc)` yields the IR locations for a JVM instruction, `origin_of(location)` the program counter for an IR location, and either may be empty.
Erased stack/local operations may have no IR node; block parameters, landing pads, and unwind arms may have no origin.
For coverage, mark the locations of each covered program counter, and never infer coverage for a location with no origin.

## Path conditions

`PathCondition::analyze` computes a disjunctive-normal-form `PathCondition` per reachable block: a disjunction of conjunctions of signed `Predicate`s.
`disjuncts` yields the conjunctions, `predicates` the referenced predicates, and `is_contradiction` reports `⊥`; blocks reachable only under a contradiction are omitted.
`&` and `|` compose structurally; `reduce`, or `analyze_with_budget` with a `SolvingBudget`, applies semantic minimization.
