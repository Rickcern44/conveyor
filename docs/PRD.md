# Conveyor

## Product requirements document

**Status:** Draft  
**Audience:** developers who want a repeatable, auditable issue-to-pull-request workflow across repositories  
**Initial implementation:** Rust CLI

## 1. Summary

Conveyor is a local-first developer workflow tool that turns a well-specified GitHub issue into a controlled agent work session and, when the required gates pass, a draft pull request.

It is deliberately not an autonomous software company. Conveyor owns workflow state, policy, evidence collection, and GitHub/Git mutations. AI agents supply planning, implementation, and review through adapters. The first adapters may invoke installed agent CLIs directly; Agent Client Protocol (ACP) is an optional adapter boundary rather than the foundation of the system.

Swan League Tracker is the first consumer, not a special case embedded in the product.

## 2. Problem

Using several coding agents is productive but leaves important coordination work manual:

- determining whether an issue is actually ready to implement;
- preserving the specification, test expectations, and review context between agents;
- consistently creating branches, commits, pull requests, and check evidence;
- knowing what needs a human decision versus what can safely proceed;
- applying the same workflow across repositories without copying one repository's conventions into another.

GitHub Projects and issues are useful sources of truth, but they do not enforce an implementation contract or provide a portable execution workflow. An agent protocol can communicate with agents, but it does not define delivery policy. Conveyor fills this orchestration layer.

## 3. Goals

1. Provide a GitHub-first, repository-agnostic path from issue to draft PR.
2. Make readiness explicit before implementation starts.
3. Keep a durable, inspectable trail of decisions, commands, test results, and agent handoffs.
4. Support multiple agent providers without giving an agent implicit authority to push, open PRs, merge, or deploy.
5. Encode each repository's commands and delivery rules in versioned configuration.
6. Work well as a small terminal program on a developer's machine.

## 4. Non-goals

- Fully autonomous production deployment or merge approval.
- Replacing GitHub Issues, GitHub Projects, CI, or code review.
- Mandating ACP, a particular LLM, or a cloud service.
- Replacing a repository's existing tests, formatter, or branch-protection rules.
- Building a multi-tenant hosted control plane in the first releases.

## 5. Users and primary jobs

### Developer / operator

Needs to prepare an issue, launch a bounded implementation workflow, inspect progress, and approve consequential Git/GitHub actions.

### Planner agent

Needs a complete issue contract and repository context to produce a plan, risks, and acceptance-test recommendations.

### Implementer agent

Needs a narrow task brief, a clean worktree/branch, repository conventions, and an explicit verification contract.

### Reviewer agent

Needs the issue contract, diff, commands run, and test output to identify scope gaps and regressions.

## 6. Workflow model

Conveyor treats GitHub as the collaborative source of truth and stores short-lived execution records locally under `.conveyor/` (ignored by Git by default). It may mirror status through GitHub labels or project fields.

```text
needs-spec → spec-ready → ready-for-build → in-progress → in-review → done
                              │                 │              │
                              └──── blocked ────┴──────────────┘
```

### State definitions

| State | Meaning | Entry requirement |
| --- | --- | --- |
| `needs-spec` | The request lacks an executable contract. | Issue exists. |
| `spec-ready` | Scope and acceptance criteria are sufficient for review. | Problem, scope, exclusions, acceptance criteria, and verification plan present. |
| `ready-for-build` | A human accepts the specification and permits implementation. | `spec-ready`; no unresolved material questions. |
| `in-progress` | Conveyor has created or attached a work session. | Clean repository and an accepted build contract. |
| `in-review` | Change and evidence are ready for human or agent review. | Required local checks complete or explicitly waived. |
| `done` | PR merged or issue otherwise closed by a human-approved rule. | GitHub state confirms completion. |
| `blocked` | Work cannot safely continue without a decision or external change. | Recorded blocker and owner. |

## 7. Issue contract

Before `ready-for-build`, an issue must contain:

- problem statement and intended user/business outcome;
- in-scope and explicitly out-of-scope behavior;
- acceptance criteria that can be verified;
- affected components or an explicit discovery task;
- test/verification expectations;
- dependencies, risks, and open questions;
- proposed delivery slice when the work is too large for one PR.

`conveyor issue validate <number>` reports missing fields and must not create a branch or start an implementer session when material requirements are absent.

## 8. Product behavior

### 8.1 Repository setup

`conveyor init` creates a versioned `.conveyor.toml` containing:

- remote and target branch;
- issue-state label/field mapping;
- branch and commit naming conventions;
- required checks and their commands;
- which actions require explicit operator confirmation;
- agent-role mapping;
- optional PR template and verification-report paths.

The configuration is committed to each consumer repository. Secrets, access tokens, and execution transcripts are not committed.

### 8.2 Readiness and handoff

`conveyor issue validate 70` fetches the GitHub issue and applies repository policy. `conveyor handoff 70 --role implementer` compiles a portable task packet containing the contract, relevant repository guidance, branch name, and required checks.

### 8.3 Work sessions

`conveyor build 70` must:

1. verify that the issue is `ready-for-build`;
2. verify a clean, up-to-date worktree;
3. create a conventional branch;
4. transition the issue to `in-progress`;
5. start the selected implementer adapter with a bounded task packet;
6. retain command, tool, and outcome metadata in a local session record;
7. after a clean implementation, push the branch and create a draft PR.

The default is draft-PR delivery after a clean implementation. Push and PR
creation remain explicit, confirmed operator actions; `--yes` is the
non-interactive equivalent, not a policy bypass. Local verification and agent
review are not prerequisites for this initial draft so CI can be the primary
verification gate.

### 8.4 Verification

`conveyor verify` remains an optional local check. It emits both human-readable
output and structured JSON, recording commands, exit status, duration,
revision, and whether results were skipped or waived. When the current branch
has a draft PR, Conveyor posts the human-readable report as a PR comment after
operator confirmation; `--yes` accepts that confirmation for automation.

### 8.5 Review and PR preparation

`conveyor review 70` packages the issue contract, diff summary, and verification report for a reviewer adapter or human reviewer. `conveyor pr open 70` remains a manual, review-gated delivery path: it creates a **draft** PR only after configured checks pass or a named operator records a waiver, and attaches a generated delivery summary to the PR body. The normal `build` path creates the initial draft earlier.

### 8.6 Status

`conveyor status` reports active sessions, issue state, current branch, required checks, blocker records, and the next allowed action. It should be sufficient to resume a session after switching between Codex, Claude, or a human developer.

## 9. Agent integration

Conveyor uses role-based adapters:

```text
Conveyor core → role adapter → agent runtime
                ├─ direct CLI adapter (first)
                └─ ACP adapter (optional)
```

The core owns state transitions and side effects. An adapter can return plans, code changes, findings, artifacts, and status; it cannot independently override policy.

ACP support should use the official Rust SDK and remain behind a feature flag until the protocol surface needed by Conveyor is stable. Direct adapters remain first-class so the tool works with local agent CLIs without a protocol bridge.

## 10. CLI surface (initial)

```text
conveyor init
conveyor doctor
conveyor status [issue]
conveyor issue validate <number>
conveyor handoff <number> --role <planner|implementer|reviewer>
conveyor build <number> [--agent <name>] [--dry-run|--yes]
conveyor verify [--format human|json] [--yes]
conveyor review <number> [--agent <name>]
conveyor pr open <number>
```

Initial release can ship `init`, `doctor`, `status`, and `issue validate` before it runs any agent or mutates GitHub.

## 11. Configuration sketch

```toml
[conveyor]
version = 1
target_branch = "main"

[github]
repository = "owner/repository"

[workflow]
ready_label = "ready-for-build"
blocked_label = "blocked"
draft_prs_only = true
require_confirmation_for = ["push", "open_pr", "merge", "deploy"]

[agents]
planner = "claude"
implementer = "codex"
reviewer = "claude"

[[checks]]
name = "unit tests"
command = "dotnet test"
required = true
```

## 12. Technical direction

Conveyor will be a Rust workspace with a thin CLI and testable core.

```text
crates/
  conveyor-cli       command parsing and terminal presentation
  conveyor-core      workflow state machine, policy, task packets
  conveyor-github    GitHub read/write adapter
  conveyor-git       worktree and branch adapter
  conveyor-verify    command execution and evidence reports
  conveyor-agents    direct and ACP adapters
```

Likely initial libraries: `clap`, `serde`, `toml`, `thiserror`, `tokio`, and a GitHub client. Library choices will be confirmed during implementation rather than frozen by this PRD.

## 13. Safety and authority model

- Read-only commands are allowed by default.
- Branch creation, issue-label changes, commits, pushes, and PR creation are distinct actions.
- Push, PR creation, merge, deployment, and destructive cleanup require explicit operator consent unless a repository policy deliberately permits otherwise.
- Every external mutation records the actor, timestamp, target, and evidence in the session record.
- Conveyor never sends repository contents or secrets to an agent beyond the adapter's configured scope.
- Configuration cannot silently downgrade a required confirmation in an active session.

## 14. Success measures

- A prepared issue can be validated and handed to an agent without manual copy/paste of requirements.
- An operator can determine why an issue is blocked and what action is permitted from one status command.
- A draft PR has a traceable issue link, and optional local verification evidence is visible in a PR comment.
- Adding a second repository requires configuration, not a fork of Conveyor.
- A user can change the planner, implementer, or reviewer runtime without rewriting workflow policy.

## 15. Delivery plan

### Milestone 0 — foundation

- Rust workspace, CLI skeleton, configuration schema, and `doctor`.
- Repository discovery and safe local session storage.
- Tests for configuration and workflow transitions.

### Milestone 1 — readiness

- GitHub issue fetch and contract validator.
- Label/project-field mapping and dry-run state transitions.
- Human-readable and JSON validation reports.

### Milestone 2 — local delivery loop

- Branch/worktree creation and verification runner.
- Task-packet generation.
- Direct CLI adapter for one implementer runtime.

### Milestone 3 — controlled PR flow

- Evidence report, reviewer handoff, and draft-PR preparation.
- Explicit confirmation flow for push and PR creation.

### Milestone 4 — optional ACP

- ACP adapter behind a feature flag.
- Session/resume support where the chosen agent runtime supports it.

## 16. Open decisions

1. Which GitHub project fields, if any, should be treated as canonical alongside labels?
2. Should the first direct adapter target Codex, Claude Code, or a generic command-template adapter?
3. Should a session record be SQLite, JSON files, or both (JSON first is preferred for inspectability)?
4. What is the minimum confirmation UX: interactive prompt only, or an explicit `--yes` flag for automation?
5. Which checks are globally standard versus entirely repository-owned?

## 17. Acceptance criteria for the first implementation slice

- `conveyor init` produces a valid configuration without overwriting an existing one.
- `conveyor doctor` identifies missing GitHub authentication, Git repository state, and configured executables without making mutations.
- `conveyor issue validate <number>` returns a clear pass/fail report against the issue contract.
- Core workflow transitions are unit tested, including disallowed transitions.
- No command in the first slice creates branches, changes issues, pushes, or opens PRs.
- The project builds and its tests run through documented Rust commands.
