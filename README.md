# MokaPot

[![Cargo Build & Test](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml/badge.svg)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
[![codecov](https://codecov.io/gh/henryhchchc/mokapot/graph/badge.svg?token=6M09J26KSM)](https://codecov.io/gh/henryhchchc/mokapot)
[![Crates.io](https://img.shields.io/crates/v/mokapot)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot)](https://docs.rs/mokapot)

MokaPot is a Java bytecode analysis library written in Rust.

> [!WARNING]
> **API Stability:** This project is in an early development stage and breaking changes can happen before v1.0.0.
> Documentations are incomplete, which will be added when the basic functionalities works.
> Using this project for production is currently NOT RECOMMENDED.

## Documentation

The documentation of the released version is available at [docs.rs](https://docs.rs/mokapot).
The documentation of the latest commit is available at [github.io](https://henryhchchc.github.io/mokapot/mokapot/)

## Usage

### Adding the dependency

Add the following line to the `[dependencies]` section in your `Cargo.toml`.

```toml
mokapot = "0.10"
```

Alternatively, to follow the latest commit version, add the following line instead.
Before building your project, run `cargo update` to fetch the latest commit.

```toml
mokapot = { git = "https://github.com/henryhchchc/mokapot.git" }
```

### Parsing a class

```rust
use mokapot::jvm::class::Class;

fn parse_class() -> Result<Class, Box<dyn std::error::Error>> {
    let reader: std::io::Read = todo!("Some reader for the byte code");
    let class = Class::from_reader(reader)?;
    Ok(class)
}
```

### MokaIR

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
