# Conveyor

Conveyor is a local-first Rust CLI for moving a well-specified GitHub issue
toward a controlled implementation workflow.

## Local development

Requirements: Rust 1.85 or newer, Git, and the GitHub CLI (`gh`).

```sh
cargo fmt --check
cargo clippy --locked --workspace --all-targets -- -D warnings
cargo test --locked --workspace
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace --all-features
cargo run -p conveyor-cli --bin conveyor -- --help
```

CI runs these checks on Linux with the stable Rust toolchain, including rustfmt
and Clippy, on pull requests targeting `main` and pushes to `main`. It can also
be started manually. The checks are named `Format`, `Rust (default)`, and
`Rust (all-features)`; a failure appears in the pull request's checks with logs
for the failing step. Tests use local fixtures and need no agent or provider
credentials.

## Workflow commands

### Interactive console

Use the local console when you want a compact operational view and command
surface rather than a sequence of separate commands:

```sh
conveyor tui
```

It lists local sessions and recent audit events, and lets you expand the
selected session with `Enter`. `j`/`k` moves between sessions and `r` reloads
local state. Type `/` to open the command palette: it accepts every existing
Conveyor command without the leading `conveyor`, for example:

```text
/issue scaffold docs/issues/login.md
/issue create --title "Add login" --body-file docs/issues/login.md
/build 70
/verify --yes
/pr open 70
```

The palette sends commands through the same parser, policy checks, and
confirmation prompts as the ordinary CLI. Conveyor restores the normal terminal
for command output and consent prompts, then returns to the console.

### Agent event output

ACP-backed commands render compact human updates by default. Use the versioned
JSONL stream for another local process or a future UI consumer:

```sh
conveyor --event-format jsonl build 70
```

Normalized, bounded event summaries are also recorded under
`.conveyor/events/<issue>.jsonl`. These records include event identity,
timestamps, issue and ACP-session context, category, and summary; they never
store raw tool output or full agent transcripts.

### Local control plane

`conveyor init` also creates `.conveyor/conveyor.db`, the local SQLite
control-plane store. JSON artifacts remain available for inspection and export:

```sh
conveyor export
```

Pending human decisions are durable records. Inspect and answer them through
the CLI today; the local TUI and forthcoming local daemon API will use these
same records:

```sh
conveyor decision list
conveyor decision answer 12 allow_once
conveyor decision answer 12 denied
```

Conveyor is intentionally the decision authority. Future phone or desktop
notifications may alert you about a request, but cannot authorize a GitHub,
Git, or agent action by themselves.

`conveyord` is the initial local control-plane process. It listens only on the
repository-local Unix socket `.conveyor/conveyord.sock` and currently exposes
health and durable-decision operations. It is intentionally narrow while the
TUI and CLI transition to the shared local API:

```sh
cargo run -p conveyor-cli --bin conveyord
```

Initialize a consumer repository with versioned, safe defaults:

```sh
conveyor init --repository owner/repository --target-branch main
```

Inspect the local repository, configured delivery remote, GitHub authentication,
and configured agent executables without changing any state:

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

## Creating and adapting issues

Create a local issue body in the required format, complete its placeholders,
then create a GitHub issue only after Conveyor confirms it is ready:

```sh
conveyor issue scaffold docs/issues/oauth-login.md
# edit docs/issues/oauth-login.md
conveyor issue create --title "Add OAuth login" --body-file docs/issues/oauth-login.md
```

For an existing issue that predates Conveyor, use the configured planner agent
to produce a validated proposal. This only writes a local draft by default;
review it before deliberately replacing the GitHub issue body:

```sh
conveyor issue adapt 70
conveyor issue adapt 70 --apply
```

Use `--yes` with `create` or `adapt --apply` only for deliberate automation.

`conveyor build 70` lists its branch, label, session, and agent actions before
asking for operator confirmation (or accepting explicit `--yes` for
automation). It creates the configured conventional branch, moves the issue
from `conveyor:ready` to `conveyor:in-progress`, writes a local session and task
packet, then runs the configured implementer.

Add `--dry-run` to perform every preflight check and display the proposed
branch, label transition, and adapter without changing Git, GitHub, local
session files, packets, or agent state.

After the implementer finishes with a clean worktree, `build` pushes the branch
and opens a draft PR by default. The build confirmation includes those delivery
actions; use `--yes` for deliberate non-interactive automation. Local
verification and AI review are no longer prerequisites for the initial draft,
so CI can be the primary verification gate.

```sh
conveyor build 70
conveyor build 70 --dry-run
conveyor build 70 --yes
conveyor build retry 70 --yes
conveyor status 70
conveyor block 70 --reason "Waiting for API access" --owner platform-team
conveyor resume 70 --to in-progress
conveyor verify --yes
conveyor review 70
conveyor pr open 70
conveyor audit 70
```

`verify` is an optional local check. It writes structured evidence to
`.conveyor/reports/latest.json` by default; configure
`verification_report_path` to use another repository-local path. When the
current delivery branch already has a PR, it also posts the human-readable
report as a PR comment after confirmation (`--yes` accepts that confirmation
for automation). If no PR exists, verification still completes and records
local evidence.
`review` creates a review packet and optionally runs the configured reviewer.
`pr open` remains available for a deliberate, review-gated draft-PR delivery:
it requires passing required checks (or `--waive-by <operator>`), shows the
push and draft-PR plan, and requests consent unless `--yes` is supplied. It
never creates a non-draft PR. `audit` prints the JSONL record of completed
branch, issue, agent, push, and PR actions for the issue.
Named verification waivers are retained in the structured evidence and audit
trail as well as in the draft PR body.

Use `block` to record an interruption and its decision owner locally. `resume`
requires an explicit next workflow state, clears that blocker, and records the
decision in the audit trail; it does not rerun an agent automatically.
When a build was interrupted after branch creation but before its implementer
ran, use `build retry` from that existing branch after resolving the recorded
blocker and restoring the configured in-progress label.

Use `--agent <name>` on `build` or `review` to select another configured
`[[acp_agents]]` or `[[agent_commands]]` adapter for that one invocation. The
configured role default remains in effect when the option is omitted.

The default branch convention is `feat/{issue}-build`. Configure a repository
specific pattern in `.conveyor.toml`; it must include `{issue}` and contain no
whitespace:

```toml
[conveyor]
remote = "origin"
branch_pattern = "work/{issue}/implementation"
commit_message_pattern = "feat: issue #{issue}"
pr_template_path = ".github/pull_request_template.md" # optional
verification_report_path = ".conveyor/reports/latest.json"

[workflow]
# These are the defaults. Override them when this repository uses different labels.
ready_label = "conveyor:ready"
in_progress_label = "conveyor:in-progress"
blocked_label = "conveyor:blocked"
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

## ACP agent integration

For a first-class Agent Client Protocol (ACP) connection, build or install
Conveyor with the optional feature:

```sh
cargo install --path crates/conveyor-cli --features acp --bin conveyor
```

Then use `codex` or `claude` as an agent role. Conveyor includes zero-install
presets that launch the corresponding ACP adapter with `npx`; no
`[[acp_agents]]` block is required for those two names. Conveyor starts the
process over stdio, initializes ACP v1, opens a session rooted at the
repository, and sends the task or review packet as the prompt. The agent may
edit the worktree, while Conveyor remains responsible for branches, issue
labels, verification, pushes, and draft PRs.

```toml
[agents]
implementer = "codex-acp"
reviewer = "codex-acp"

[[acp_agents]]
name = "codex-acp"
program = "npx"
args = ["-y", "@agentclientprotocol/codex-acp@latest"]
```

The explicit configuration above remains useful to pin an adapter version or
use a different program. With the built-in preset, use this instead:

```toml
[agents]
implementer = "codex"
reviewer = "claude"
```

Conveyor records the ACP session ID in `.conveyor/sessions/<issue>.json`. Send
a follow-up to the same agent conversation with:

```sh
conveyor agent continue 70 "Address the reviewer feedback, then run the checks."
```

During an ACP turn, Conveyor prints agent messages, plans, and tool lifecycle
updates. When the agent requests a sensitive operation, Conveyor shows the
specific request and offers only a one-time approval. Declining the prompt (or
having no one-time option) cancels that operation. `--yes` authorizes the
Conveyor command itself; it never silently approves later ACP permission
requests.

Run `conveyor doctor` after configuring this: it checks the configured process
executable (`npx` in this example). If an ACP agent is configured in a binary
without ACP support, Conveyor stops with the exact reinstall command rather
than falling back to a direct command adapter.
