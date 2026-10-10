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
