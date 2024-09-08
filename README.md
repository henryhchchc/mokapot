# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?style=flat-square&logo=GitHub)](https://github.com/henryhchchc/mokapot)
[![CI - GitHub Actions](https://img.shields.io/github/actions/workflow/status/henryhchchc/mokapot/ci.yml?style=flat-square&logo=githubactions&logoColor=white&label=CI)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)
[![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?style=flat-square&logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/)
[![Crates.io](https://img.shields.io/crates/v/mokapot?style=flat-square&logo=rust&logoColor=white)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot?style=flat-square&logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot)
[![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?style=flat-square&logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md)

MokaPot is a Java bytecode analysis library written in Rust.

> [!WARNING] > **API Stability:** This project is in an early development stage and breaking changes can happen before v1.0.0.
> Documentations are incomplete, which will be added when the basic functionalities works.
> Using this project for production is currently NOT RECOMMENDED.

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
