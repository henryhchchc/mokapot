# Contribution Guide

## Bug Report

For bug report, please open an issue at the [GitHub issue tracker](https://github.com/henryhchchc/mokapot/issues).

## Pull Request

Please follow the [Conventional Commits](https://www.conventionalcommits.org/) when writing commit messages.
It would be nice to include the scope in the commit message.
Generally, the scope will be the name of the top level module.
For changes in `src/x`, the scope will be `x`.
For example, when making changes to `src/jvm/class.rs`, the commit message will be `feat(jvm): xxx` or `fix(jvm): xx`.

Before submitting a pull request, please do the following checks:

- Make sure `cargo fmt --check` does not complain.
- Make sure `cargo clippy --all-targets --all-features -- -D warnings` does not complain.

## Tasks

MokaPot needs your contribution to be better. Please check [TODO.md](TODO.md) for a list of tasks that we are planning to do.

## Testing

`tests/jdk_classes.rs` contains integration test that runs MokaPot on JDK classes.
Additional steps are required to run the test case.
```bash
# First, tell MokaPot to run integration tests.
export INTEGRATION_TEST=1
# Then, extract the JDK classes from the JDK distribution.
jimage extract --dir="<extraction path>" "$JAVA_HOME/lib/modules"
# Next, tell MokaPot where the extracted JDK classes are.
export JDK_CLASSES="<extraction path>"
# Finally, run the integration test.
cargo nextest run --run-ignored=all
```

## Developer Certificate of Origin

By contributing to this project, you must certify that your contribution complies with the [Developer Certificate of Origin](https://developercertificate.org).
You may use the following `git` command to [sign-off](https://git-scm.com/docs/git-commit#Documentation/git-commit.txt--s) your commit.

```bash
git commit --signoff
```
