# Conveyor

Conveyor is a local-first Rust CLI for moving a well-specified GitHub issue
toward a controlled implementation workflow.

## Local development

Requirements: Rust 1.85 or newer, Git, and the GitHub CLI (`gh`).

```sh
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo run -p conveyor-cli --bin conveyor -- --help
```

## First-slice commands

Initialize a consumer repository with versioned, safe defaults:

```sh
conveyor init --repository owner/repository --target-branch main
```

Inspect the local repository, GitHub authentication, and configured agent
executables without changing any state:

```sh
conveyor doctor
```

Validate a GitHub issue that uses the Conveyor issue sections (`Problem`, `In
scope`, `Out of scope`, `Acceptance criteria`, `Affected components`,
`Verification plan`, `Dependencies and risks`, and `Open questions`):

```sh
conveyor issue validate 70
conveyor issue validate 70 --format json
```

The current slice does not create branches, change GitHub issues, push, or
open pull requests.
