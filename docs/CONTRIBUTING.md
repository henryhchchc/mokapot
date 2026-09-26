# Contributing to MokaPot

We welcome all contributions to MokaPot!
Whether you're fixing bugs, adding features, improving documentation, or sharing feedback, your help is appreciated.

For questions or help, open an issue or start a [GitHub Discussion](https://github.com/henryhchchc/mokapot/discussions).

## Bug Reports and Feature Requests

- **Bug Reports:** Open an issue at the [GitHub issue tracker](https://github.com/henryhchchc/mokapot/issues).
  Include details: steps to reproduce, expected and actual behavior, environment info.

- **Feature Requests:** Open an issue describing your idea and motivation.
  Always check [existing issues](https://github.com/henryhchchc/mokapot/issues) to avoid duplicates.

## Code Contributions

- Fork the repository and create a branch from `main`.
- Write clear, conventional commit messages.
  Follow [Conventional Commits](https://www.conventionalcommits.org/).
  Include a scope (top-level module, e.g. `feat(jvm): ...` for changes in `src/jvm`).
- Make sure code is correctly formatted.
  Run `cargo fmt --check`.
- Make sure `clippy` does not report any warnings.
  Run `cargo clippy --all-targets --all-features -- -D warnings`.
- Add or update tests as needed.
- For integration tests, see instructions below.
- Push your branch and open a Pull Request (PR) against `main`.
- Respond to review feedback and update your branch as needed.
- PRs are merged after passing checks and review.

## Testing

- Unit tests live beside the code they cover.
- Java fixture tests live in `tests/` and run with the default test suite.
  They require JDK 27 (`javac` and `jar`).
- The JDK smoke test in `tests/jdk_classes.rs` is ignored by default.

```bash
# Run unit and Java fixture tests
cargo test --all-features

# Run tests without compiling or running Java fixtures
MOKAPOT_SKIP_JAVA_TESTS=1 cargo test --all-features

# Extract JDK classes from your JDK distribution
jimage extract --dir="<extraction path>" "$JAVA_HOME/lib/modules"

# Set the path for extracted JDK classes
export JDK_CLASSES="<extraction path>"

# Run only the ignored JDK smoke test using its optimized profile
MOKAPOT_SKIP_JAVA_TESTS=1 cargo nextest run --cargo-profile=jdk-smoke --all-features --test jdk_classes --run-ignored=all
```

## Developer Certificate of Origin (DCO)

Certify compliance with the [Developer Certificate of Origin](https://developercertificate.org) for all contributions.

Sign off commits:

```bash
git commit --signoff
```

Thank you for contributing to MokaPot!
