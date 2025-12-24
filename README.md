# MokaPot

[![GitHub Repository](https://img.shields.io/badge/GitHub-henryhchchc%2Fmokapot-orange?logo=GitHub)](https://github.com/henryhchchc/mokapot)
[![Codecov](https://img.shields.io/codecov/c/github/henryhchchc/mokapot?logo=codecov&logoColor=white&label=Coverage)](https://app.codecov.io/gh/henryhchchc/mokapot/)
[![Crates.io](https://img.shields.io/crates/v/mokapot?logo=rust&logoColor=white)](https://crates.io/crates/mokapot)
[![docs.rs](https://img.shields.io/docsrs/mokapot?logo=docsdotrs&logoColor=white&label=docs%2Frelease)](https://docs.rs/mokapot)
[![Contributor Covenant](https://img.shields.io/badge/Contributor_Covenant-2.1-4baaaa?logo=contributorcovenant)](docs/CODE_OF_CONDUCT.md)
[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/henryhchchc/mokapot)

## Overview

MokaPot is a Rust library for working with JVM bytecode. You can use it to parse, inspect, and change Java class files.

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

## Contributing

See [docs/CONTRIBUTING.md](docs/CONTRIBUTING.md) for how to contribute.

## License

MIT License. See [LICENSE](LICENSE) for details.
