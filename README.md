# MokaPot

[![Cargo Build & Test](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml/badge.svg)](https://github.com/henryhchchc/mokapot/actions/workflows/ci.yml)

MokaPot is a Java byte code analysis library to facilitate my research.

> [!NOTE]
> This project is in an early development stage and stability is not the current focus.
> I will add the documentation stuff when the basic functionalities are ready for use.

## Usage

### Adding the dependency

To use the latest development version, add the following line to the `[dependencies]` section in your `Cargo.toml`.

```toml
mokapot = { git = "https://github.com/henryhchchc/mokapot.git" }
```

Before building your project, run `cargo update` to fetch the latest commit.

### Parsing a class

```rust
use mokapot::elements::ClassParser;

let mut reader: std::io::Reader = todo!("Some reader for the byte code");
let class = ClassParser::new(&mut reader).parse()?;
```
