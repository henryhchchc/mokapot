# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?logo=GitHub)](https://github.com/henryhchchc/mokapot) [![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/) [![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot) [![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot) [![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md) [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/henryhchchc/mokapot)

## Overview

MokaPot is a Rust library for working with JVM bytecode.
You can use it to parse, inspect, and change Java class files.

For library usage and API documentation, see the [mokapot crate](crates/mokapot/).

## Documentation

- [Release documentation](https://docs.rs/mokapot)
- [Latest commit documentation](https://henryhchchc.github.io/mokapot/mokapot/)

## Building

Requirements:

- Rust (latest stable)
- JDK (latest release, for compiling Java source files as test data)

To build and test:

```sh
cargo build --all-features
cargo test --all-features
```

## Benchmarks

The parsing benchmarks use an extracted JDK image, shared with the JDK smoke tests.
Extract it once:

```sh
jimage extract --dir=./jdk_classes "$JAVA_HOME/lib/modules"
export JDK_CLASSES="$PWD/jdk_classes"
```

Run raw class parsing and whole-class parsing on shard 0 of 16:

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo bench -p mokapot --bench jdk-parsing --all-features
```

Each iteration parses the selected classes sequentially and drops each result.
Discovery, file reads, input validation, and fingerprinting happen outside timing.
Results include latency per corpus pass, byte throughput, and classes per second.
The raw parser leaves attribute payloads unresolved; whole-class parsing also decodes attributes and instructions and resolves raw elements into the public model.

Selection uses a fixed hash of module-relative class paths, independent of CPU count and extraction location.
Set `JDK_BENCH_SHARD_INDEX` to select another shard or `JDK_BENCH_SHARD_COUNT=1` to process the whole corpus (with index unset or 0).
Only the selected input bytes are retained in memory.
Full-corpus runs need more memory and take longer.
The benchmark prints the class count, byte count, and corpus fingerprint so comparisons can verify identical inputs.

To validate every case without collecting timings:

```sh
MOKAPOT_SKIP_JAVA_TESTS=1 cargo bench -p mokapot --bench jdk-parsing --all-features -- --test
```

Filter cases with `-- raw_class` or `-- whole_class`.
Record the JDK version and use the same corpus fingerprint, Rust toolchain, features, and machine when comparing revisions.
These benchmarks require no compiled Java test fixtures.
When `JDK_CLASSES` is unset, the benchmarks are ignored during ordinary test and benchmark runs.
Listing benchmarks does not load the corpus.

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for how to contribute.

## License

MIT License.
See [LICENSE](LICENSE) for details.
