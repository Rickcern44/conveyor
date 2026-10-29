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

The first target is a useful closed loop on macOS for one operator, one existing
repository and one worker. A Linux dev box, remote runners and multiple workers
are future deployment options, not prerequisites. Measure accepted work against
the operator time spent defining, supervising, reviewing and repairing it, plus
the time spent maintaining Conveyor.

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

1. Discuss and define a task using the canonical issue contract. The controller
   validates the proposal and displays scope, exclusions, checks, actions and
   limits. Material unanswered questions prevent approval.
2. The operator explicitly approves the version and configuration snapshot.
   Execution starts only within that authority; changed terms return to approval.
3. The controller records the run and supervises the agent. A request outside
   its authority becomes **awaiting operator**; an external prerequisite may
   leave it **blocked**. A specific valid decision or resolved prerequisite may
   allow the same run to resume only if the runtime supports it and the approval
   is still valid. Otherwise record interruption and start a separately approved
   attempt as needed.
4. Verification produces revision-bound evidence. A failed required check means
   **failed** unless an authorized, bounded repair succeeds. A named waiver is
   visible as a waiver, not a pass. A draft can exist during verification.
5. **Review-ready** requires the approved verification contract and delivery
   evidence; a draft PR alone is insufficient. **Completed** means the agreed
   verified outcome is accepted by the operator, with delivery state recorded.
   Failure and cancellation are terminal for that run; a new run can be approved
   against an appropriate version. Cancellation stops owned work, preserves
   recoverable artifacts and never implies a successful result. A blocked or
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
OpenCode is a planned ACP provider. Unavailable models and unsupported controls
must produce clear errors; never silently switch models or incur paid usage as a
fallback. Evaluate conversational model candidates on representative tasks,
latency, tool reliability, corrections and cost before choosing defaults. Role
naming and exact provider capabilities remain design decisions.

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
Its initial goal is an interactive macOS task-to-verified-result loop, followed
by durable daemon ownership, reconnect and conversational handoff. A draft PR
may be created early, but review-ready requires the approved verification
contract and evidence for the resulting revision. Failed or interrupted work
must remain inspectable and recoverable; retries reconcile delivery state.

Evaluate the product with representative real tasks, measuring accepted work,
operator effort, rework, failures and maintenance against a manual baseline.
The pilot repository, runtime, enforceable limits and model defaults remain open
operator decisions. Stronger container-backed environments, a dedicated dev box,
remote and multiple workers, scheduling, notifications, webhooks and graphical
interfaces are later options driven by demonstrated need.
