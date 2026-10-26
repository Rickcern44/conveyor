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

## Workflow commands

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

`conveyor build 70` lists its branch, label, session, and agent actions before
asking for operator confirmation (or accepting explicit `--yes` for
automation). It creates the configured conventional branch, moves the issue
from `ready-for-build` to `in-progress`, writes a local session and task
packet, then runs the configured implementer.

```sh
conveyor build 70
conveyor status 70
conveyor block 70 --reason "Waiting for API access" --owner platform-team
conveyor resume 70 --to in-progress
conveyor verify
conveyor review 70
conveyor pr open 70
conveyor audit 70
```

`verify` writes structured evidence to `.conveyor/reports/latest.json`.
`review` creates a review packet and optionally runs the configured reviewer.
`pr open` requires passing required checks (or `--waive-by <operator>`), shows
the push and draft-PR plan, and requests consent unless `--yes` is supplied.
It never creates a non-draft PR. `audit` prints the JSONL record of completed
branch, issue, agent, push, and PR actions for the issue.
Named verification waivers are retained in the structured evidence and audit
trail as well as in the draft PR body.

Use `block` to record an interruption and its decision owner locally. `resume`
requires an explicit next workflow state, clears that blocker, and records the
decision in the audit trail; it does not rerun an agent automatically.

Use `--agent <name>` on `build` or `review` to select another configured
`[[agent_commands]]` adapter for that one invocation. The configured role
default remains in effect when the option is omitted.

The default branch convention is `feat/{issue}-build`. Configure a repository
specific pattern in `.conveyor.toml`; it must include `{issue}` and contain no
whitespace:

```toml
[conveyor]
branch_pattern = "work/{issue}/implementation"
commit_message_pattern = "feat: issue #{issue}"
```

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
