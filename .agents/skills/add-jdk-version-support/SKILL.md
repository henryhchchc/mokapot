---
name: add-jdk-version-support
description: Locate the code, configuration, and checks involved in adding JDK version support to Mokapot.
---

# Add JDK version support

## Where to modify

Paths are relative to `crates/mokapot/`, except CI:

- `src/jvm/class/mod.rs`: `Version`, `Version::LATEST` (latest non-preview version), `from_versions`, `major`, `minor`, and `is_preview_enabled`.
  The upper major-version bound is derived from `Version::LATEST.major()`; tests and default test classes use `LATEST` too.
- `src/intrinsics/macros.rs`: `see_jvm_spec!`'s `__latest_jdk` arm for rustdoc specification links.
- `.github/workflows/ci.yml`: shared `JAVA_VERSION` and JDK distribution.
- `src/jvm/bytecode/class_file/`: class-file parsing and generation, if the format changes.
- `src/jvm/` and `src/ir/`: models, bytecode decoding, and IR conversion affected by specification changes.
- `src/jvm/class/tests.rs`: version parsing and round-trip tests.
- `test_data/mokapot/`: Java fixtures for new compiler output.

## What to check

- Target release's JVM specification and relevant JEPs: major/minor versions, preview rules, and format or bytecode changes.
- Version tests: ordinary and preview round-trips, invalid minors, and unsupported majors.
- Rustdoc: specification links resolve; `cargo doc --all-features --no-deps` succeeds.
- CI: the configured JDK distribution provides the target release.
- Java fixtures: `build.rs` uses `javac` and `jar` from `PATH` and does not explicitly track JDK changes.
  Select the target JDK in both `JAVA_HOME` and `PATH`, force fixture rebuilding when switching JDKs, and run without `MOKAPOT_SKIP_JAVA_TESTS`.
- Repository checks: see `AGENTS.md`.
- JDK corpus: `tests/jdk_classes.rs` covers parsing, decoding, IR construction, bounded path-condition analysis, and writing/reparsing.
  These tests are ignored in ordinary runs.
  Set `JDK_CLASSES` to a fresh extraction directory and run:

```sh
jimage extract --dir="$JDK_CLASSES" "$JAVA_HOME/lib/modules"
MOKAPOT_SKIP_JAVA_TESTS=1 cargo nextest run --cargo-profile=jdk-smoke --all-features --test jdk_classes --run-ignored=all
```
