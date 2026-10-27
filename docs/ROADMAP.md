# Conveyor future plan

This roadmap extends Conveyor from a local-first CLI into a human-controlled
workflow control plane. The core principle remains unchanged: Conveyor owns
state transitions, delivery policy, audit, and Git/GitHub mutations; agents
perform bounded planning, implementation, and review work.

## Current baseline

The CLI has an issue contract, configurable workflow labels, local sessions
and audit records, direct-command and ACP agent adapters, verification
evidence, review packets, and draft-PR delivery. ACP turns now produce
normalized terminal events and ask for one-time permission decisions.

## Phase 1 — durable agent event stream

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

## Phase 2 — SQLite control-plane storage

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

## Phase 3 — chat-ops interaction and notifications

Introduce a notification abstraction for events that need a human decision:

- ACP permission or elicitation requests;
- blocked sessions and named decision owners;
- failed required verification;
- draft PR ready for approval;
- retry/recovery actions after partial Git or GitHub mutations.

Start with Slack because it supports authenticated interactive controls,
threads, and team-level routing. Discord and Telegram should use the same
adapter interface for smaller teams or personal workflows. Email and generic
webhooks remain notification-only fallback channels.

Chat messages should contain a minimal summary and links to GitHub and local
artifacts, never source files, secrets, or full agent transcripts. They can
offer narrow actions such as **Approve once**, **Deny**, **View status**, or
**Open draft PR**, but every action must be signed, tied to a pending decision,
and revalidated by Conveyor before execution.

Each notification needs deduplication, acknowledgement state, escalation
timeout, and an audit entry. A notification never authorizes an action by
itself: the operator must make a separate explicit decision in Conveyor.

Acceptance criteria:

- An ACP permission request creates one expiring, auditable chat approval card.
- Only configured and authenticated operators can answer it.
- Duplicate delivery, an expired button, or an altered payload cannot execute
  an action.
- The CLI remains a complete fallback when no chat integration is configured.

## Phase 4 — optional local UI

A local UI becomes valuable later for dense operational views rather than as
the first approval surface. It should consume the same SQLite-backed event
stream and use the same Conveyor decision API as chat integrations.

Useful first views are an issue/session queue, event timeline, verification and
review readiness, file-change summaries, and GitHub links. The UI sends intent
to Conveyor; it must never hold GitHub credentials or communicate directly with
an agent adapter.

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
4. Build Slack first, then Discord/Telegram adapters, with signed expiring actions.
5. Add a local scheduler for read-only reminders and reconciliation.
6. Add authenticated GitHub webhook intake and durable event processing.
7. Build a local operational UI only when chat and CLI views no longer provide
   sufficient visibility.
