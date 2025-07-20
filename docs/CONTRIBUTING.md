# Contributing to MokaPot

We welcome all contributions to MokaPot! Whether you're fixing bugs, adding features, improving documentation, or sharing feedback, your help is appreciated.

For questions or help, open an issue or start a [GitHub Discussion](https://github.com/henryhchchc/mokapot/discussions).

## Bug Reports and Feature Requests

- **Bug Reports:**
  Open an issue at the [GitHub issue tracker](https://github.com/henryhchchc/mokapot/issues). Include details: steps to reproduce, expected and actual behavior, environment info.

- **Feature Requests:**
  Open an issue describing your idea and motivation. Always check [existing issues](https://github.com/henryhchchc/mokapot/issues) to avoid duplicates.

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

- Unit tests are scattered throughout the codebase.
- Integration tests are in the `tests/` directory.
- `tests/jdk_classes.rs` contains integration tests for JDK classes.

```bash
# Enable integration tests
export INTEGRATION_TEST=1

# Extract JDK classes from your JDK distribution
jimage extract --dir="<extraction path>" "$JAVA_HOME/lib/modules"

# Set the path for extracted JDK classes
export JDK_CLASSES="<extraction path>"

# Run the integration tests
cargo nextest run --run-ignored=all
```

## Developer Certificate of Origin (DCO)

Certify compliance with the [Developer Certificate of Origin](https://developercertificate.org) for all contributions.

Sign off commits:

```bash
git commit --signoff
```

Thank you for contributing to MokaPot!
