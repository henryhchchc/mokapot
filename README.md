# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?logo=GitHub)](https://github.com/henryhchchc/mokapot) [![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/) [![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot) [![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot) [![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md) [![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/henryhchchc/mokapot)

MokaPot is a Rust library for parsing, inspecting, and analyzing JVM bytecode.
It provides a Java class-file model and MokaIR, an SSA intermediate representation for analyzing method behavior.

Use MokaPot to:

- Read class files, including bytecode instructions and attributes.
- Analyze control flow and data flow with MokaIR.
- Build custom tools for JVM bytecode.

## Getting Started

```sh
cargo add mokapot
```

See the [crate guide](crates/mokapot/README.md) for usage and the [examples](crates/mokapot/examples/) for sample tools.

## Documentation

- [Release documentation](https://docs.rs/mokapot)
- [Latest commit documentation](https://henryhchchc.github.io/mokapot/mokapot/)
- [MokaIR guide](docs/MokaIR.md)

## Contributing

See the [contributing guide](docs/CONTRIBUTING.md) for development, testing, and benchmarks.

## License

MIT License.
See [LICENSE](LICENSE) for details.
