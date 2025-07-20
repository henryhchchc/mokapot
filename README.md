# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?logo=GitHub)](https://github.com/henryhchchc/mokapot)
[![CI - GitHub Actions](https://img.shields.io/github/actions/workflow/status/henryhchchc/mokapot/ci.yml?logo=githubactions&logoColor=white&label=CI)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
[![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/)
[![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot)
[![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/henryhchchc/mokapot)

MokaPot is a library for analyzing and manipulating JVM bytecode. It is written in Rust for performance and safety.

## Documentation

The documentation of the released version is available at [docs.rs](https://docs.rs/mokapot).
The documentation of the latest commit is available at [github.io](https://henryhchchc.github.io/mokapot/mokapot/)

## Usage

### Adding the dependency

Run the following command in the root directory of your project.

```sh
cargo add mokapot
```

Alternatively, to follow the latest commit version, run the following command instead.
Before building your project, run `cargo update` to fetch the latest commit.

```sh
cargo add --git https://github.com/henryhchchc/mokapot.git mokapot
```

### Parsing a class

```rust
use mokapot::jvm::class::Class;

fn parse_class() -> Result<Class, Box<dyn std::error::Error>> {
    let reader = todo!("Some reader for the byte code");
    let class = Class::from_reader(&mut reader)?;
    Ok(class)
}
```

### More Examples

More example usage of MokaPot can be found in the [examples](examples/) folder.

### MokaIR

> [!WARNING]
> **API Stability:** MokaIR is currently considered unstable. APIs and implementations are subject to breaking changes.

MokaIR is an intermediate representation of JVM bytecode in [mokapot](https://github.com/henryhchchc/mokapot).
To learn more, please refer to [docs/MokaIR.md](docs/MokaIR.md)

## Building

Make sure you have the following tools installed:

- The latest stable version of Rust
- The latest release version of JDK

Compile the project and run the tests with the following command.

```bash
cargo build --all-features
cargo test --all-features
```

## Contributing

Cool. Contributions are welcomed. See the [contribution guide](docs/CONTRIBUTING.md) for more information.
