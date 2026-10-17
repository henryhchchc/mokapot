# Contributing to MokaPot

Check [existing issues](https://github.com/henryhchchc/mokapot/issues) before reporting bugs or proposing features.
For bugs, include reproduction steps, expected and actual behavior, and your environment.
Ask questions in [GitHub Discussions](https://github.com/henryhchchc/mokapot/discussions).

## Pull Requests

- Branch from `main` and open a PR against `main`.
- Add tests for behavior changes and describe the change and validation in the PR.
- Use [Conventional Commits](https://www.conventionalcommits.org/) with a module scope, such as `feat(jvm): ...`.
- Sign off commits with `git commit --signoff` to certify the [Developer Certificate of Origin](https://developercertificate.org).

AI-assisted contributions are welcome.
Human contributors must review and take responsibility for submitted code.
Do not credit AI agents as authors or co-authors in source files, commit messages, or metadata.

## Development

Use the latest stable Rust and JDK 27 (`javac` and `jar` for Java fixtures).
Run from the repository root:

```sh
cargo build --all-features
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
```

Format Markdown with `rumdl fmt`.
Unit tests live beside their source; integration tests and Java fixtures live in `crates/mokapot/tests` and `crates/mokapot/test_data`.
To skip compiling and running Java fixtures:

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo test --all-features
```

## JDK Smoke Tests and Benchmarks

Both use extracted JDK classes.
With `JAVA_HOME` set, extract them once:

```sh
jimage extract --dir=./jdk_classes "$JAVA_HOME/lib/modules"
export JDK_CLASSES="$PWD/jdk_classes"
```

Run the ignored smoke test with its optimized profile (requires `cargo-nextest`):

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo nextest run --cargo-profile=jdk-smoke --all-features --test jdk_classes --run-ignored=all
```

Run parsing benchmarks:

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo bench -p mokapot --bench jdk-parsing --all-features
```

The `raw_class` case leaves attribute payloads unresolved; `whole_class` also decodes attributes and instructions and resolves the public model.
Each iteration parses and drops classes sequentially; discovery, reads, validation, and fingerprinting are outside timing.
Results report latency per corpus pass, byte throughput, and classes per second.

By default, benchmarks select shard 0 of 16 using a fixed hash of module-relative paths.
Set `JDK_BENCH_SHARD_INDEX` to select another shard, or `JDK_BENCH_SHARD_COUNT=1` for the full corpus (index unset or 0).
Only selected bytes remain in memory; full-corpus runs require more memory and time.
For comparisons, record the JDK version and use matching corpus fingerprints, Rust toolchains, features, and machines.

Append `-- --test` to validate cases without timing, or `-- raw_class` / `-- whole_class` to filter cases.
Benchmarks need no Java fixtures and are ignored when `JDK_CLASSES` is unset.
Listing benchmarks does not load the corpus.

Run IR generation and path-condition benchmarks:

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo bench -p mokapot --bench jdk-ir
MOKAPOT_SKIP_JAVA_TESTS=1 cargo bench -p mokapot --bench jdk-path-conditions
```

Both reuse the parsing benchmark's corpus selection, sharding, and fingerprint.
Neither requires `bench-internals` or Java fixtures.
`jdk-ir` parses classes and validates IR generation before timing construction of IR for every method with a body.
`jdk-path-conditions` builds IR before timing `PathCondition::analyze` with its default solving budget.
Each iteration processes a complete corpus or bin sequentially, including dropping each result.
Results report latency per pass and methods per second; file reads, parsing, validation, and bin selection are outside timing.

Path-condition cases have one bin per observed predicate-occurrence count, ordered numerically, with empty bins omitted.
The count matches the JDK smoke tests: visit reachable blocks once and sum predicates across conditional successor guards.
An ordinary `if` contributes two occurrences; a switch default can contribute several.
This is a workload proxy, not a count of distinct Boolean variables.
`JDK_BENCH_MAX_PREDICATES` sets the inclusive maximum (default 16; zero selects only methods without predicate occurrences).
Corpus and method summaries follow the results table, including bin populations and the number of excluded methods.

Append `-- --test` to validate each case without timing or `-- --list` to list cases.
Listing path-condition cases with `JDK_CLASSES` set performs corpus setup to discover the bins.
Use an exact filter such as `-- --exact 'jdk_path_conditions::analyze::2'` to select one bin.
Without `JDK_CLASSES`, neither target loads inputs or runs benchmarks.

### Continuous Benchmarking with CodSpeed

The benchmarks use `codspeed-divan-compat`, so `cargo bench` behaves as plain divan locally.
The `CodSpeed` workflow runs them on every push to `main` and on pull requests in CPU simulation mode, using shard 0 of 64 of the JDK corpus.
To reproduce a CodSpeed run locally (requires `cargo-codspeed` and the `codspeed` CLI):

```sh
export MOKAPOT_SKIP_JAVA_TESTS=1 JDK_BENCH_SHARD_COUNT=64
cargo codspeed build -m simulation -p mokapot --features bench-internals
codspeed run -m simulation -- cargo codspeed run -p mokapot
```
