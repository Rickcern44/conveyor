# Conveyor delivery roadmap

Status: planned; baseline inspected 2026-10-29.

## Product outcome

Conveyor is a personal software factory: discuss a change, approve a bounded
task, leave it running, and return to a result that is easy to review. Success
means more useful work accepted with less operator attention, including time
spent maintaining Conveyor.

The conversational agent investigates, proposes tasks, asks focused questions,
and explains results. The deterministic controller owns authoritative state,
approvals, execution limits, verification, recovery, and delivery. Execution
agents perform bounded work through replaceable adapters.

The first release supports one existing repository and one execution worker.
Rust, ACP, the CLI, and the existing TUI remain the foundation. Interactive and
unattended operation use the same application services and policies.

## Current baseline

Implemented foundations:

- Issue-contract validation, task/review packets, workflow transitions, and
  repository configuration.
- Direct-command and optional ACP agent adapters; Git/GitHub draft-PR delivery.
- Local JSON sessions, audit records, verification reports, and review packets.
- Bounded ACP event summaries in JSONL and SQLite.
- SQLite migrations and pending-decision storage.
- A TUI session/activity dashboard and slash-command dispatch.
- A repository-local daemon socket with health and decision operations.

Important gaps:

- Sessions remain JSON records; SQLite does not yet own execution state.
- The CLI owns agent execution; the daemon does not supervise runs or recover
  them. The TUI returns to ordinary terminal output while commands execute.
- ACP permissions still wait on a terminal prompt. Stored decisions are not yet
  an asynchronous channel back to a waiting agent, and expiry is not enforced.
- Sessions are associated with issues rather than versioned tasks and distinct
  execution attempts. Approval is not bound to an immutable task version.
- Builds use the current checkout; dedicated worker environments and resource
  limits are not implemented.
- Initial draft PRs can precede verification. Direct adapters do not yet have
  normalized event parity with ACP.

These foundations are reusable, but a schema or socket endpoint alone does not
count as a delivered execution capability.

## Deployment assumptions

- Develop and prove the workflow locally before installing it on the dev box.
- First dev-box target: Ubuntu Server LTS, one worker, and remote model providers
  through an authenticated agent runtime. Confirm hardware, OS, and repository
  requirements during M0; provision only when access is available.
- Run controller and worker on the dev box. Access the CLI/TUI over SSH; closing
  SSH must not stop approved work. Keep the control API on a local Unix socket.
- Retain repository-local SQLite initially. Multi-repository aggregation and a
  laptop controller dispatching to another host are later extensions.
- Begin with one supported ACP runtime, while preserving the direct-adapter
  interface and current CLI paths.
- Hardware provisioning and account authentication are explicit operator steps;
  never put credentials into repository configuration or task artifacts.

## Milestone sequence

| Milestone | Delivered capability | Exit demonstration |
| --- | --- | --- |
| M0 | Agreed handoff and first deployment contract | One real pilot task and a documented execution policy |
| M1 | Durable tasks, attempts, approvals, and events | Restart reconstructs the same task and run state |
| M2 | Background execution and durable decisions | Disconnect, reconnect, and answer a pending request |
| M3 | Dedicated worker and enforced limits | One bounded run preserves work through interruption |
| M4 | Verification, repair, and reviewable delivery | A real task produces a PR with revision-bound evidence |
| M5 | Dev-box installation and operations | SSH disconnect, reboot, and backup/restore drills pass |
| M6 | Conversational handoff in the TUI | Discuss, approve, execute, and request changes in one surface |
| M7 | Personal pilot and release decision | Repeated real use shows useful output and measured effort |

The sequence is dependency-driven, not a calendar commitment. M5 host preparation
can proceed after M0; unattended operation requires M1–M4. Each milestone closes
only when its exit demonstration passes.

## M0 — define the handoff contract

Deliverables:

- Update the PRD around the solo operator and the discuss → define → approve →
  execute → verify → review workflow. Reconcile older first-slice language in
  contributor guidance with the implemented product.
- Specify Task, TaskVersion, Run, AgentSession, Decision, Evidence, and Delivery.
  One task can have multiple runs, and a run can use multiple agent sessions.
- Specify approval bound to the task version, configuration snapshot, permitted
  actions, and limits. Changed scope requires a new version and approval.
- Define terminal outcomes, operator questions, cancellation behavior, and what
  constitutes verified completion. A draft PR alone is not completion.
- Select one consumer repository, one agent runtime, and a small representative
  issue. Record the dev-box inventory, access method, and required build tools.

Exit gate: an example approved task includes scope, exclusions, acceptance
criteria, verification commands, delivery authority, time/retry limits, and
clear reasons to interrupt the operator. Unresolved choices become explicit
requirements rather than implicit agent decisions.

## M1 — establish durable task and run state

Deliverables:

- Migrate SQLite to store task versions, runs, agent-session links, approval
  snapshots, transitions, blockers, evidence references, and delivery attempts.
- Import existing JSON sessions with explicit legacy identity; preserve JSON
  exports and artifacts. Define SQLite as authoritative for new managed runs
  rather than maintaining two competing authoritative stores.
- Commit transitions and their event records transactionally; retain stable
  event IDs and add task/run identity plus ordered reconnect cursors.
- Use an outbox or equivalent reconciliation record for Git/GitHub mutations;
  process failure must not imply a mutation never happened.
- Enforce decision expiry and single resolution, bound to the relevant run and
  request. Export tasks, runs, decisions, events, and configuration snapshots.

Exit gate: close and reopen the store, recover identical state, reject stale or
duplicate decisions, and replay events without duplicated effects. Migration
and export round trips have meaningful automated coverage.

## M2 — make the daemon own execution

Deliverables:

- Extract application operations from terminal-specific command handlers.
  CLI, TUI, and future conversational tools call the same operations.
- Introduce a versioned local API for submit/start, status, event subscription,
  decisions, cancellation, and results, with structured errors and request IDs.
- Have the daemon supervise agent processes and persist lifecycle transitions.
  ACP permission/elicitation requests wait on durable decisions rather than
  terminal input. Unsupported direct-adapter interaction fails explicitly.
- Reconnect clients from an event cursor; render compact expandable activity
  and a decision inbox in the TUI. CLI parity remains available for automation.
- On daemon restart, reconcile recorded runs with processes and workspaces.
  Mark interrupted work accurately; resume only where the runtime supports it.

Exit gate: start an approved run, close the TUI, reconnect from the CLI, answer
its permission request, and observe the same outcome in both clients. A daemon
crash creates neither silent approval nor a second active execution.

## M3 — deliver one bounded worker environment

Deliverables:

- Create a dedicated checkout/worktree per run; record base revision,
  environment configuration, workspace location, and produced revision.
- Enforce one active worker, wall-clock and repair-attempt limits, cancellation,
  and termination of the owned process group. Add memory/CPU limits for the
  Linux execution environment where supported.
- Define controller/worker credentials and execution boundaries. A worktree is
  checkout separation, not a security sandbox. For unattended use, select and
  test an OS-user/container boundary appropriate to the pilot repository;
  keep delivery credentials and host control sockets outside worker access.
- Record usage/cost when the runtime provides it. Do not promise a hard spend
  limit without an enforceable provider/runtime mechanism; report unavailable
  accounting explicitly and enforce the available time/attempt limits.
- Preserve patches and evidence on failure. Make workspace cleanup deliberate
  and retention configurable.
- Complete required normalized lifecycle/outcome event parity for direct and
  ACP adapters, documenting runtime-specific limitations.

Exit gate: a task cannot alter the operator checkout, exceed its enforceable
limits, or leave an untracked owned process after cancellation. Killing the
agent preserves recoverable work and records a truthful outcome.

## M4 — return verified, reviewable results

Deliverables:

- Run controller-selected checks from the approved verification contract;
  agents cannot declare a failed check successful or silently weaken criteria.
- Bind evidence to the exact resulting revision. Treat repository build/test
  commands as untrusted execution and apply the worker boundary to them too.
- Support a bounded implement → verify → repair loop. Exhausted limits produce
  a useful failed/blocked result, not unbounded retries.
- Distinguish draft-created, verifying, blocked/failed, and review-ready states.
  CI may be a gate, but the controller must observe its result for the relevant
  revision before claiming review-ready.
- Deliver the PR link, change summary, acceptance-criteria evidence, check
  results, unresolved concerns, and preserved artifacts. Explicit waivers remain
  visible and distinct from passing verification.
- Reconcile remote branches and PR identity before retrying delivery; retain
  recorded approval for only the actions it actually authorized.

Exit gate: complete one real pilot task. A failed required check prevents
review-ready status. Interrupt delivery after a remote mutation and retry;
reconciliation returns the existing PR rather than creating another one.

## M5 — install and prove the dev box

Deliverables:

- Document the host setup, supported agent/runtime versions, repository tools,
  authentication steps, and a repeatable binary install/update procedure.
- Install the daemon as a systemd-managed service with an appropriate service
  identity, restart policy, local socket permissions, state paths, and logs.
  Choose a credential mechanism that works without an interactive login.
- Run one worker with the M3 isolation and resource limits. Install supporting
  repository services only where the pilot needs them.
- Provide health/doctor checks for agent authentication, GitHub delivery access,
  disk space, build tools, and worker readiness.
- Document safe service stop, update, compatible rollback, and schema migration
  recovery. Back up SQLite using a consistent snapshot method; include needed
  configuration and artifacts, and test restoration on a separate directory.
- Keep remote access through SSH initially. A TUI running on the dev box over
  SSH is sufficient; distributed runner registration is not a prerequisite.

Exit drills, with recorded results:

1. From the laptop, SSH in, approve a pilot task, disconnect, and reconnect to
   its completed result or an accurately recorded request for attention.
2. Reboot during execution; the service starts and reconciles interrupted work
   without duplicate execution, delivery, or lost approval boundaries.
3. Kill an agent, exhaust a time limit, and expire a decision; each preserves
   useful work and produces the expected outcome.
4. Restore a backup to a separate location and inspect tasks and evidence.
5. Re-run the M4 delivery interruption drill on the installed host.

Passing these drills marks the first unattended dev-box release, even before
the conversational interface is complete.

## M6 — add conversational handoff to the TUI

Deliverables:

- Add a persistent project conversation alongside task cards, an attention
  inbox, expandable activity, and a compact result view.
- Give the planner structured operations for proposing/versioning tasks,
  requesting approved execution, reading status, requesting changes, and
  cancelling runs. Validate every operation in the controller.
- Start by translating existing issues and conversation into the same handoff
  contract; support explicit scope review before execution.
- Distinguish answers to blockers from scope changes. Neither conversation
  history nor agent assertions are authoritative execution state.
- Expose the same capabilities through CLI/API paths. Slash commands provide
  predictable navigation and actions without independent policy logic.

Exit gate: discuss a small change, inspect its proposed contract, approve it,
disconnect, and return to the result. Request changes as another bounded run.
A fresh conversation reconstructs progress from stored records.

## M7 — validate personal productivity

Use the dev box for a pilot of at least five real tasks spanning a bug fix, a
small feature, and a maintenance change. Record task size and failed attempts
as well as accepted work.

Measure accepted tasks, operator time spent defining/supervising/reviewing/
repairing, task duration, interruptions, rework/regressions, and Conveyor
maintenance time. Compare with a small recorded sample of the existing manual
workflow. Set acceptable thresholds in M0; a five-task pilot is an initial
signal rather than proof of general productivity gains.

Exit gate: document observed benefit, failure patterns, and a deliberate
decision to improve reliability, expand the pilot, or add capacity. Retain
human review and approval of merge/deployment.

## Deferred expansion

After the unattended workflow and pilot are useful:

- Authenticated remote runners, a laptop-hosted controller, and multiple hosts.
- Multiple repositories/workers with scheduling and resource accounting.
- Optional ntfy notifications pointing back to authoritative decisions/results.
- Timers, GitHub webhook intake, deduplication, and reconciliation.
- Additional chat integrations, browser UI, and richer editing capabilities.

These remain valid extensions of the same task/run API. They are not required
to prove the first personal dev-box workflow.

## Implementation discipline

- Deliver each milestone in reviewable slices with behavior-level tests for
  policy, persistence, retries, and failure recovery; run repository checks.
- Use fake agents and GitHub/Git fixtures for deterministic interruption tests,
  then repeat the critical drills against the real pilot environment.
- Preserve bounded event summaries and avoid secret-bearing transcripts.
- Approval applies only to recorded scope and actions. `--yes` never grants
  future agent permissions; notifications and external triggers grant none.
- Review required permissions and verification on every new task version.
- Keep status truthful: planned, running, interrupted, blocked, failed, and
  review-ready have distinct meanings. A daemon restart need not resume an
  agent conversation automatically to count as safe recovery.

## Next implementation slice

Begin M0 with the PRD and a concrete example handoff contract, then implement
the M1 task/version/run model and migration tests. This supersedes the previous
plan to mirror the existing issue-session shape into SQLite unchanged.
