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

Read-only commands never create branches or change GitHub. `conveyor build`
lists its branch, label, session, and agent actions before asking for operator
confirmation (or requiring explicit `--yes` for automation). Pushes and pull
requests are not implemented yet.

## Direct local agent adapter

Configure an explicit command template before running `conveyor build`. The
template is invoked directly (without a shell) only after build preflight and
operator confirmation pass.

```toml
[agents]
implementer = "local-implementer"

[[agent_commands]]
name = "local-implementer"
program = "./scripts/conveyor-implementer"
args = ["--packet", "{packet_path}"]
```

Supported placeholders are `{packet_path}`, `{repo_root}`, `{issue_number}`,
`{branch}`, and `{role}`. The packet instructs the agent not to perform Git or
GitHub delivery actions; Conveyor retains that authority.
