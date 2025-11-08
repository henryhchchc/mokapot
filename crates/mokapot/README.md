# MokaPot

[![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot)

## Overview

MokaPot is a Rust library for working with JVM bytecode. You can use it to parse, inspect, and analyze Java class files.

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
See [docs/MokaIR.md](../../docs/MokaIR.md) for details.

## Contributing

See the [project repository](https://github.com/henryhchchc/mokapot) for contributing guidelines.

## License

MIT License. See [LICENSE](../../LICENSE) for details.
