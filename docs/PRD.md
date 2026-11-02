# Conveyor product requirements

Status: working contract, 2026-10-29. The [roadmap](ROADMAP.md) defines the
sequence and exit demonstrations; this document distinguishes current behavior
from requirements that have yet to be enforced.

## Outcome and boundary

Conveyor is a local-first personal software factory for a solo operator: discuss
a change, define and approve a bounded task, execute it, verify the result and
return a reviewable delivery. The operator retains merge and deployment
judgment. The controller owns policy, state, approvals and delivery; agents
propose and perform bounded work through replaceable adapters. A draft PR is a
delivery artifact, not proof of verified completion.

The first V0 pilot runs locally on the operator's Mac against a planned,
Swan-like test repository containing a .NET API, React/Vite frontend, and Aspire
orchestration. Its backend-only task is to implement
`GET /api/health/details`, returning the application name and version with
tests. The repository and task have not been provisioned. V0 uses simulated
delivery and produces a local review packet; it does not prove real GitHub
behavior or unattended operation. A later real-world trial uses Swan League
commissioner participation work and is separate from the test pilot. A Linux
dev box, SSH/reboot validation, and real draft-PR smoke tests are optional later
work, not prerequisites. Measure accepted work against operator time spent
defining, supervising, reviewing, and repairing it, plus Conveyor maintenance.

## Implemented baseline

The Rust CLI and core validate issue contracts, create task and review packets,
manage configured workflow transitions, and use direct command and optional ACP
adapters. The CLI supports issue authoring/adaptation, local sessions, audit
records, verification reports and draft-PR delivery. The TUI displays sessions
and dispatches commands. SQLite stores events and pending decisions; a local
Unix-socket daemon exposes health and decision operations. README documents the
current commands and consent prompts. Current builds use the operator checkout,
and may create a draft before local verification or review. Verification can
post a PR comment with consent. The explicit `pr open` path requires passing
configured checks or a named waiver. These features do not yet constitute an
unattended, version-bound execution service.

Read-only commands are the default. State-changing commands follow their
configured policy and confirmation rules; `--yes` is explicit consent to the
command's disclosed actions, not a policy bypass or consent to later agent
permission requests. Issue creation or replacement, branch and label changes,
push, draft-PR creation and PR comments require the applicable operator
confirmation. An agent cannot grant itself authority. Merge and deploy remain
operator decisions and are not automated by this contract. A waiver is a named
operator decision and is recorded distinctly from a passing check.

## Planned handoff model

These are requirements, not claims about current JSON sessions or SQLite:

- **Task:** controller-owned durable identity for a requested outcome, linked
  to its source issue when one exists; one task has one or more versions and runs.
- **TaskVersion:** immutable identity and scope revision of a task, including
  exclusions, acceptance criteria and verification contract. Approval applies
  to a particular version.
- **Run:** a uniquely identified bounded attempt against one task version and
  configuration snapshot. A task can have multiple runs; each has its own state,
  limits, decisions, evidence and deliveries.
- **AgentSession:** a uniquely identified role/runtime interaction linked to
  a run; a run can have multiple sessions. The runtime supplies session identity;
  the controller records the link and retains authority.
- **Decision:** a uniquely identified operator resolution of a specific request,
  bound to its run, version and permitted action. It is single-use and expires
  when specified; neither an agent nor a notification can approve it.
- **Evidence:** controller-recorded verification or review, tied to the produced
  revision and run, including failed, skipped and waived checks. A waiver is
  attributed to a named operator and never represented as a pass.
- **Delivery:** controller-recorded identity and status of branch, commit and PR
  actions, their authorization and reconciliation evidence, linked to a run.
  Retry reconciles remote state before repeating an external mutation.

Approval binds the immutable TaskVersion, configuration snapshot, permitted
actions and execution limits. A change to scope, configuration or limits
requires a new approval for the updated version or snapshot. Approval cannot be
inferred from conversation, prior runs, agent assertions or `--yes` on another
command. These version-bound guarantees and durable expiry are planned, not yet
implemented.

## Planned lifecycle and outcomes

The desired entry point is `conveyor task create "Task to do"`. A human-facing
planning agent and operator build the structured spec and execution plan in
digestible stages: outcome/scope, behavior/questions, verification, then
execution/authority. The agent summarizes each stage and continues unless
corrected. During planning it may read code and documentation, but must not
change code or run checks. It proposes focused tests/checks for inclusion in the
approved plan. Initially, planning may happen in an external agent; saving and
resuming an unfinished planning conversation is future work.

After material questions are resolved, the controller presents the full spec
and plan for one explicit final approval. That approval authorizes the complete
listed workflow: scoped implementation, the agreed checks, review, and only the
delivery actions named in the plan. Approval binds the immutable TaskVersion,
workflow/configuration snapshot, permitted actions, and limits. Any change to
scope, checks, configuration, or limits requires new approval. After approval,
clean checks and review proceed automatically without a mandatory human review
gate. In-scope review findings can use the shared repair attempt, followed by
fresh verification and review. Pause for a material out-of-scope decision, new
authority, or a blocker unresolved within approved limits. The operator inspects
the resulting packet/PR; merge and deployment remain operator decisions.

Runtime permission requests covered by the approved plan may be granted only
when the controller matches the specific requested operation to that authority.
Unknown, ambiguous, or out-of-scope requests pause for the operator. Record each
request and decision; there is no silent blanket permission, and `--yes` is not
a general bypass.

The V0 workflow is `prepare → implement → verify → review → simulate-deliver`.
One repair attempt is shared across verification failures and in-scope review
findings. A changed revision must pass verification and review again. V0 allows
30 minutes of active execution, excluding human waits when no executor is
running. Exhaustion returns a truthful failed/blocked result and preserves work.

V0 review-ready means the approved verification and review contract passed for
the candidate revision and a local simulated-delivery packet was created. The
packet is not a real PR. Later real delivery requires reconciled PR identity.
The packet contains proposed PR title/summary, diff, base/target and candidate
revisions, check results, review findings, unresolved concerns, and a simulated
delivery identity/idempotency record. Repeatable controller/CI evaluations use
fresh fixture repositories and deterministic fake agents; separate real-model
Luna runs measure variable outcomes, time, and available cost. Containers are
optional. Never inject artificial defects into real delivery.
Failure and cancellation are terminal for that run; a new run requires approval.
Cancellation stops owned work and preserves recoverable artifacts. A blocked or
awaiting-operator run is not terminal and must not silently resume or approve.

The [example contract](example-task-contract.md) walks through these cases.

## Planned repository onboarding and environments

Onboarding inspects repository metadata without executing setup: language
manifests, lockfiles, toolchain versions, dev-container definitions and service
declarations. Conveyor proposes a reusable blueprint with evidence and
uncertainties, and asks the operator to resolve ambiguous choices. The operator
reviews the selected environment, setup commands and checks before execution;
re-detection proposes a diff rather than silently overwriting configuration.

A blueprint supplies defaults for runtimes, versions, setup and verification
commands, services, environment-variable names, execution mode and cleanup.
Repository configuration records the chosen blueprint and explicit overrides.
Secrets and credentials remain outside configuration and task artifacts.
Start with a native macOS blueprint for the pilot; existing dev-container
definitions may be supported through a later compatible adapter.

Each managed run uses a dedicated worktree and records its base revision and
configuration. A worktree separates checkouts, not operating-system permissions:
native agents run with the user's access. Before unattended work is treated as
isolated, select and test an execution boundary covering credentials, host files
and control sockets. The first local loop may remain foreground and interactive;
daemon-owned execution, disconnect and recovery are later requirements.

## Planned operator experience and agent configuration

The TUI is the first operator interface, supported by a CLI and API that share
the same controller and policy. The intended TUI includes a project conversation,
task list, decision inbox, compact timeline with expandable activity and output,
and result view. It shows stages, agent activity, commands, changed files,
checks, decisions and failures without requiring a full transcript. Persist
bounded event summaries and make detailed output retention configurable. A
future graphical client should use the same services; reassess its value after
using the TUI for real tasks.

The conversational agent may investigate, propose a task, ask questions and
explain results through structured controller operations. It cannot approve its
own proposal, change an approved scope or declare a failed check successful.
The operator may answer a blocker without changing scope; a material scope
change requires a new approval. A fresh conversation reconstructs status from
controller records rather than relying on conversation history.

Agent roles should independently select a provider and model, with optional
provider-specific controls only where supported. Preserve compatibility with
existing configuration and specify precedence for defaults and run overrides.
For the pilot, implementation and review are separate role configurations,
both selecting Luna through Codex ACP. Confirm exact model identifiers and
adapter support before implementation. If unavailable, fail visibly; never
silently switch models/providers or incur unapproved cost. OpenCode is a later
planned ACP provider. Evaluate conversational model candidates on representative
tasks, latency, tool reliability, corrections and cost before choosing defaults.
Role naming and exact provider capabilities remain design decisions.

## Planned workspace management

List Conveyor-managed worktrees with their task, run, status, last activity,
path and approximate disk usage where available. Explain that shared Git objects
and external caches make estimates incomplete. The CLI and TUI should offer a
preview and explicitly confirmed manual cleanup of a selected managed worktree.
Never target the primary checkout or worktrees Conveyor does not own. Refuse to
remove active work or work that is uncommitted, unpushed, unreconciled or
otherwise not preserved, and explain how to preserve it. Use Git-aware removal
and report reclaimed space only when measurable.

An optional age-based retention policy may follow, disabled by default and
configurable per repository or blueprint. It must recheck ownership, activity
and preservation at deletion time and record an audit event. Define last
activity and behavior when remotes cannot be reached before enabling it; age
alone does not establish that work is safe to discard.

## Delivery, success and deferred expansion

The [roadmap](ROADMAP.md) is authoritative for sequencing and exit demonstrations.
Its initial goal is a local macOS task-to-simulated-result loop, followed by
durable daemon ownership, reconnect and conversational handoff. A local packet
is not a real PR. Real draft-PR delivery is a later explicitly authorized
validation. Review-ready requires the approved verification/review contract
and evidence for the resulting revision. Failed or interrupted work must remain
inspectable and recoverable; retries reconcile delivery state.

Evaluate repeatability with fresh fixture repositories, deterministic scripted/
fake agents and simulated delivery for CI and controller recovery/idempotency
tests. Run separate real-model Luna evaluations to measure variable outcomes,
time and available cost. Never introduce artificial defects into a real
delivery. The later Swan League commissioner participation task is a distinct
real-world trial. Stronger container-backed environments, a dedicated dev box,
remote and multiple workers, scheduling, notifications, webhooks and graphical
interfaces are later options driven by demonstrated need.
