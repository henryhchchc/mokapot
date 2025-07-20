# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?logo=GitHub)](https://github.com/henryhchchc/mokapot)
[![CI - GitHub Actions](https://img.shields.io/github/actions/workflow/status/henryhchchc/mokapot/ci.yml?logo=githubactions&logoColor=white&label=CI)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
[![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/)
[![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot)
[![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/henryhchchc/mokapot)

## Overview

MokaPot is a Rust library for working with JVM bytecode. You can use it to parse, inspect, and change Java class files.

Main features:

- Parse JVM bytecode
- Work with an intermediate representation (MokaIR)
- Build custom tools for JVM bytecode
- Includes documentation and examples

## Documentation

- [Release documentation](https://docs.rs/mokapot)
- [Latest commit documentation](https://henryhchchc.github.io/mokapot/mokapot/)

## Installation

To add MokaPot to your project, run:

```sh
cargo add mokapot
```

To use the latest commit from GitHub:

```sh
cargo add --git https://github.com/henryhchchc/mokapot.git mokapot
cargo update
```

## Usage

### Parse a JVM class file

```rust
use mokapot::jvm::class::Class;
use std::fs::File;

fn parse_class_file(path: &str) -> Result<Class, Box<dyn std::error::Error>> {
    let mut file = File::open(path)?;
    let class = Class::from_reader(&mut file)?;
    Ok(class)
}
```

### More Examples

See the [examples](examples/) directory for more code samples.

### MokaIR

MokaIR is an intermediate representation of JVM bytecode in this library.
See [docs/MokaIR.md](docs/MokaIR.md) for details.

## Building

Requirements:

- Rust (latest stable)
- JDK (latest release, for compiling Java source files as test data)

To build and test:

```sh
cargo build --all-features
cargo test --all-features
```

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for how to contribute.

## License

MIT License. See [LICENSE](LICENSE) for details.
