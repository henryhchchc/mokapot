# Repository Guidelines

## Project Structure & Module Organization
This repository is a Cargo workspace with the main crate in `crates/mokapot`. Core library code lives under `crates/mokapot/src`, organized by domain: `jvm/` for class-file parsing and models, `ir/` for Moka IR and analysis backends, `analysis/` for higher-level analyses, and `types/` for descriptors and signatures. Integration tests live in `crates/mokapot/tests`, while unit-style module tests sit alongside source files under `src/**/tests`. Java fixtures used by Rust tests are stored in `crates/mokapot/test_data`.

## Build, Test, and Development Commands
Run commands from the repository root:

- `cargo build --all-features` builds the workspace with optional features enabled.
- `cargo test --all-features` runs the default local test suite.
- `cargo fmt --all -- --check` verifies formatting.
- `cargo clippy --all-targets --all-features -- -D warnings` matches CI lint strictness.
- `cargo run --example disassembler -- <class-file>` runs the example in `crates/mokapot/examples/disassembler`.
- `cargo hack check --feature-powerset --no-dev-deps` checks feature combinations used in CI.

## Coding Style & Naming Conventions
Use Rust 2024 edition defaults and keep code `rustfmt`-clean. The crate enables strict lints in `src/lib.rs`, including `clippy::pedantic`, `missing_docs`, and broken intra-doc link denial, so public APIs should be documented and warning-free. Follow existing naming patterns: `snake_case` for modules, files, and functions; `PascalCase` for types; focused module names like `class_loader`, `method_descriptor`, and `fixed_point`.

## Testing Guidelines
Add unit tests near the code they cover and integration tests in `crates/mokapot/tests/<feature>.rs`. If tests depend on compiled Java fixtures, ensure `javac` is available; `build.rs` recompiles files from `crates/mokapot/test_data`. JDK-wide ignored tests require extracted JDK classes:

- `export INTEGRATION_TEST=1`
- `export JDK_CLASSES=/path/to/jdk_classes`
- `cargo nextest run --run-ignored=all`

## Commit & Pull Request Guidelines
Use Conventional Commits with a scope that matches the top-level module or area, for example `feat(jvm): parse record attributes` or `fix(ir): guard stack frame merge`. Sign commits with `git commit --signoff` to satisfy the DCO. PRs should target `main`, describe behavior changes, list verification commands, and link issues or discussions when relevant. Include output samples or screenshots only when the change affects user-visible CLI or docs behavior.
