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

## Planned handoff model (M0 contract, M1 implementation)

These are requirements, not claims about current JSON sessions or SQLite:

- **Task:** controller-owned durable identity for a requested outcome, linked
  to its source issue; one task has one or more versions and runs.
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

1. Discuss and define a task using the canonical issue sections. The controller
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

## Planned delivery and deferred expansion

M1 makes tasks, versions, runs, decisions, evidence and transitions durable and
reconciles external actions. M2 moves execution and durable decisions into the
daemon. M3 adds a bounded worker environment; a Git worktree alone is not a
security sandbox. M4 adds revision-bound verification and reviewable delivery;
M5 proves operation on a dev box; M6 adds conversational handoff; M7 measures a
personal pilot. These are planned capabilities with milestone exit gates in the
roadmap. Multi-host/multi-worker operation, remote runners, notifications,
webhooks and richer interfaces are deferred until the single-worker pilot is
useful. The pilot repository, runtime, host and enforceable limits remain open
operator decisions.
