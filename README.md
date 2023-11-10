# MokaPot

[![Cargo Build & Test](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml/badge.svg)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
![Crates.io](https://img.shields.io/crates/v/mokapot)
![docs.rs](https://img.shields.io/docsrs/mokapot)

MokaPot is a Java byte code analysis library to facilitate my research.

> [!WARNING]
> **API Stability:** This project is in an early development stage and breaking changes can happen before v1.0.0.
> Documentations are incomplete, which will be added when the basic functionalities works.
> Using this project for production is currently NOT RECOMMENDED.

## Usage

### Adding the dependency

To use the latest development version, add the following line to the `[dependencies]` section in your `Cargo.toml`.

```toml
mokapot = { git = "https://github.com/henryhchchc/mokapot.git" }
```

Before building your project, run `cargo update` to fetch the latest commit.

### Parsing a class

```rust
use mokapot::elements::Class;

let reader: std::io::Read = todo!("Some reader for the byte code");
let class = Class::from_reader(reader)?;
```

## Documentation

The documentation of the stable version is available at [docs.rs](https://docs.rs/mokapot).
The documentation of the latest commit is available at [github.io](https://henryhchchc.github.io/mokapot/mokapot/)