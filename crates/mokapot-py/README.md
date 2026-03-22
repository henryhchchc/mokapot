# mokapot Python bindings

Initial Python bindings for selected `mokapot` JVM class APIs.

## Local development

```sh
cd crates/mokapot-py
uv sync
uv run maturin develop
```

## Tests

```sh
cd crates/mokapot-py
uv run pytest tests
```

The test session compiles Java fixtures from
`crates/mokapot/test_data/mokapot/**/*.java`, so `javac` is required.

## Usage

```python
import mokapot

cls = mokapot.Class.from_file("MyClass.class")
print(cls.binary_name)
print(cls.version.major, cls.version.minor)
print(cls.access_flags.bits)
print(cls.is_interface())
```
