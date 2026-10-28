# Conveyor future plan

This roadmap extends Conveyor from a local-first CLI into a human-controlled
workflow control plane. The core principle remains unchanged: Conveyor owns
state transitions, delivery policy, audit, and Git/GitHub mutations; agents
perform bounded planning, implementation, and review work.

## Current baseline

The CLI has an issue contract, configurable workflow labels, local sessions
and audit records, direct-command and ACP agent adapters, verification
evidence, review packets, and draft-PR delivery. ACP turns now produce
normalized, durable event summaries with JSONL output; the local TUI presents
session and activity views over the same records.

## Phase 1 — durable agent event stream ✓

Promote the CLI-local ACP event model into a public, versioned core type.
Every agent adapter should emit the same event categories:

- session lifecycle: started, resumed, completed, failed, cancelled;
- agent text and plan updates;
- tool start, progress, completion, and file-change summaries;
- permission and elicitation requests plus the selected decision;
- verification, review, delivery, blocker, and workflow-state events.

Add a JSONL renderer with stable event IDs, timestamps, issue/session IDs,
and schema versioning. The terminal renderer remains a presentation of these
events; it must not be the source of truth. Do not persist unbounded agent
transcripts by default, because they may include repository content or
secrets. Persist event summaries and explicit decisions instead.

Acceptance criteria:

- `conveyor ... --event-format jsonl` is consumable by another process.
- The human terminal renderer and JSONL stream describe the same event.
- Permission decisions are auditable without storing sensitive tool output.

## Phase 2 — SQLite control-plane storage (in progress)

JSON files remain useful portable artifacts, but use SQLite for durable,
queryable operational state once Conveyor gains long-running processes,
notifications, and inbound webhooks. SQLite is the right first storage choice:
it is local-first, transactional, inspectable, easy to back up, and does not
require a hosted service.

Use a repository-local database by default, such as `.conveyor/conveyor.db`,
with migrations and a documented schema. Keep current JSON session/evidence
artifacts exportable during migration; do not make SQLite opaque or
irrecoverable.

Initial tables:

- repositories and versioned configuration snapshots;
- sessions, workflow transitions, blockers, and delivery confirmations;
- normalized agent events and bounded summaries;
- pending/resolved permission and elicitation requests;
- notification deliveries and acknowledgements;
- scheduled jobs, idempotency keys, and run results;
- inbound webhook deliveries and reconciliation results.

Acceptance criteria:

- Interrupted processes recover pending decisions and scheduled work safely.
- Every external mutation is linked to its session, decision, and audit event.
- `conveyor export` can produce portable JSON records for support or migration.

## Phase 3 — local operator communication and runner control

For a solo local software factory, establish Conveyor as the operator
communication layer before adding third-party chat. Add `conveyord`, a local
daemon backed by SQLite and a versioned Unix-domain-socket API. The TUI is the
primary operator surface; the CLI remains a complete fallback and automation
surface. Both must call the same decision API and never implement policy
independently.

The daemon owns long-lived ACP turns, pending decisions, event fan-out, and
local runner lifecycle. Persist permission and elicitation requests with expiry,
idempotency, and selected outcome. The TUI presents a compact inbox with
explicit **Approve once**, **Deny**, and **Cancel** actions. Closing a terminal
must not lose a request or silently approve it.

Acceptance criteria:

- An ACP request creates one expiring, auditable local pending decision.
- TUI and CLI inspect and answer the same decision through the local API.
- Duplicate, expired, or altered answers cannot execute an action.
- Restarted daemon processes recover pending work safely.

## Phase 4 — remote runner and notifications

Add authenticated remote runner registration for dedicated development PCs. A
remote runner executes an ACP adapter but has no independent delivery authority;
it receives bounded work and returns the same event stream. Start with private
authenticated networking, never a public control-plane endpoint.

Once the local decision API is stable, add self-hosted `ntfy` as a
notification-only adapter for pending decisions and completed work. A
notification is never authority: it directs the operator back to Conveyor.
Slack, Discord, Telegram, email, and generic webhooks remain later optional
team integrations.

## Phase 5 — timers and scheduled control-plane work

Add a scheduler that runs read-only or policy-approved maintenance tasks:

- periodically refresh GitHub issue/PR state for active sessions;
- identify stale verification evidence or abandoned branches;
- remind owners about blocked sessions or unanswered permission requests;
- run repository-defined read-only health checks;
- queue, but do not automatically execute, consequential recovery actions.

Use durable scheduled-job records with idempotency keys, last-run results, and
per-repository concurrency limits. The scheduler should be a separate local
process/service so a terminal closing does not lose timers. It must default to
read-only behavior and require explicit repository policy for any mutation.

## Phase 6 — GitHub webhooks and remote coordination

Add an optional webhook receiver for GitHub events such as issue edits,
labels, comments, pull-request review status, merge, and check completion.

Webhook processing should:

- verify GitHub signatures before accepting an event;
- store raw delivery metadata and deduplicate by delivery ID;
- translate events into the same internal event model as ACP and CLI actions;
- reconcile local session state with GitHub without silently overriding an
  active operator decision;
- enqueue a notification or a read-only reconciliation job when action is
  needed.

Do not expose a public webhook endpoint until authentication, secret storage,
replay protection, and a deployment model are established. A local tunnel or
GitHub App installation may be appropriate for early experiments; neither
should be required for the CLI-only workflow.

## Cross-cutting guardrails

- Preserve explicit approval boundaries for push, PR, merge, deploy, and ACP
  permissions. `--yes` never grants future or agent-requested permissions.
- Keep the repository configuration authoritative for branch conventions,
  workflow labels, checks, and notification policy.
- Maintain CLI parity: every chat or UI action must have an inspectable CLI/API path.
- Version event schemas and control-plane APIs before supporting external UI
  clients.
- Treat GitHub webhooks and timers as untrusted triggers, not as authority to
  mutate repositories.
- Add contract tests for event schemas, notifications, scheduling idempotency,
  and webhook signature validation before enabling automation by default.

## Suggested next implementation order

1. Extract `AgentEvent` into `conveyor-core` and add JSONL output.
2. Introduce SQLite migrations, session/event storage, and JSON export.
3. Add persistent permission-request records and a CLI answer command.
4. Add `conveyord` plus a Unix-socket local decision API; move the TUI onto it.
5. Add persistent decision records and a CLI/TUI answer flow.
6. Add local ACP runner lifecycle, recovery, and event fan-out.
7. Add an authenticated remote runner host, then optional ntfy notifications.
8. Add a scheduler and later GitHub webhook intake after the local control
   plane is stable.
