# Conveyor product requirements

Status: proposed validation contract, 2026-11-02. The [delivery plan](ROADMAP.md)
defines implementation slices and exit demonstrations. All requirements below
are planned unless explicitly identified as implemented.

## Product outcome

Conveyor runs a human-approved software-change specification through a durable,
inspectable workflow. The human develops the spec with a planning agent before
execution. Each workflow step consumes explicit inputs, produces artifacts, and
hands off only when controller-enforced criteria are satisfied. The operator
returns to a reviewable result or an accurate explanation of where work stopped;
real PR delivery is a later authorized stage.

The first V0 pilot runs locally on the operator's Mac, against a small,
Swan-like test repository containing a .NET API, React/Vite frontend, and
Aspire orchestration. Its representative task changes only the backend:
implement `GET /api/health/details` to return the application's name and
version, with tests. The repository and task are planned and have not been
provisioned. The pilot uses simulated delivery and produces a local review
packet; it does not prove real GitHub delivery or unattended operation. A later
real-world trial uses Swan League commissioner participation work and is a
separate stage. Running on an always-on host, SSH/reboot validation, and
real-draft-PR smoke tests are optional later work, not prerequisites for V0.

The value being tested is less operator supervision and easier recovery and
review. Learning Rust and agent behavior is a separate benefit, not evidence of
productivity. Merge and deployment remain operator decisions.

## Implemented baseline and gaps

The existing CLI/core provide issue-contract validation, task/review packets,
workflow labels, direct-command and optional ACP adapters, verification reports,
audit records, and draft-PR delivery. SQLite stores events and decisions; the
local Unix-socket daemon exposes health and decision operations. The TUI shows
sessions and dispatches commands. Read-only operations are the default;
state-changing commands use their configured policy and operator confirmation.
See README for current behavior. In particular, current commands do not yet
provide the staged planning or version-bound automated workflow specified here.

Existing JSON sessions are issue-oriented. The daemon does not yet supervise
execution; ACP permissions still use terminal prompts. Builds use the operator
checkout and can open a draft before verification. These capabilities do not
provide the versioned workflow runs, step attempts, or recovery defined here.

## Human handoff

The desired first interaction is `conveyor task create "Task to do"`. A
human-facing planning agent works with the operator to create a structured spec
and execution plan. Planning proceeds in digestible stages: outcome and scope;
behavior and open questions; verification; then execution and authority. After
each stage the agent summarizes the decisions and continues unless the operator
corrects it. At the end, the controller presents the complete resolved plan for
one explicit approval. This interaction is planned; implementation may initially
stage planning in an external agent before embedding it in Conveyor. Saving and
resuming an unfinished planning conversation is future work.

During planning, the agent may freely read relevant code and documentation but
must not change code or run checks. It proposes focused tests and other checks;
the operator approves the exact verification plan before execution. The spec
uses the canonical issue sections:

- Problem, in scope, out of scope, and acceptance criteria.
- Affected components, verification plan, dependencies and risks, open questions.

The handoff adds the repository/base revision, selected workflow and overrides,
separately configured implementation and review agent selections, permitted
actions, time/attempt limits, and review policy. Each acceptance criterion maps
to a mechanical check or to a decision that must be resolved before approval.
Material unanswered questions prevent approval. Syntactic validation alone does
not prove a spec is adequate.

One explicit approval authorizes the complete listed workflow: scoped
implementation, the agreed checks, review, and the specifically authorized
delivery action. Approval binds an immutable spec version, workflow version,
configuration snapshot, permitted actions, and limits. Changes require a new
version and approval; active runs retain their original definitions. Planning
conversation is supporting context, not execution authority. After approval,
the controller proceeds end to end without a mandatory human review gate when
verification and review are clean. It pauses for a material out-of-scope
decision, a request for new authority, or a blocker that cannot be resolved
within the approved limits. The operator inspects the resulting packet/PR;
merge and deploy remain separate operator decisions.

## Workflow and composition

The first workflow is `prepare → implement → verify → review → simulate-deliver`.
One repair attempt is shared across verification failures and in-scope review
findings. A repair always returns to verification; a changed revision must pass
verification and review again. Review findings within approved scope can trigger
that repair automatically. A clean review does not wait for a mandatory human
approval; a human inspects the completed local packet afterward. Exhausting the
repair or time budget yields a truthful failed/blocked result and preserves work.
The pilot allows 30 minutes of active execution, excluding human waits when no
executor is running.

A step definition has a stable name, executor kind (code, agent, or human), typed
input/output references, completion criteria, failure/attention routing, and
limits. An agent's completion message cannot satisfy a mechanical gate by itself.
Step state and artifacts are persisted before dependent work is dispatched.

| Step | Inputs | Required output and completion criterion |
| --- | --- | --- |
| Prepare | Approved contract and repository/base revision | Dedicated workspace, recorded base/environment, successful preflight |
| Implement | Spec and prepared workspace | Recorded candidate commit and change summary; scope concerns surfaced |
| Verify | Candidate commit and approved checks | Results for each required check tied to that commit; all pass to advance |
| Repair | Failed verification and remaining allowance | New candidate commit; always returns to verification |
| Review | Passing verification, diff, acceptance mapping | Review findings and concerns tied to the candidate revision; clean review or an in-scope repair request |
| Simulate-deliver | Candidate, checks, review, approved delivery scope | Local review packet with proposed PR title/summary, diff, base/target and candidate revisions, checks, review findings, unresolved concerns, and a simulated delivery identity/idempotency record |

Review uses a separately selected review agent and may produce findings. In-scope
findings use the shared repair allowance; material scope changes pause for the
operator and require a new approved version. Any code change invalidates
downstream verification/review evidence. Simulated delivery is local evidence
only and must never be reported as a real branch or PR. The eventual product goal
remains a real draft PR; occasional explicitly authorized GitHub smoke tests in a
dedicated repository are future validation, not part of every pilot run.

The pilot configures implementation and review independently to use Luna through
the Codex ACP adapter, even though the selected model is initially the same for
both roles. Before implementation, confirm the exact model identifiers and ACP
adapter support. If either selection is unavailable, fail visibly and ask for a
decision; never silently substitute a model/provider or incur unapproved cost.
OpenCode is a possible later ACP provider, not part of this pilot.

Composition initially means resolving a versioned base definition with validated
repository overrides: verification commands, agent selection, time/repair limits,
and review requirements. Persist the resolved definition and its identity per
run. Reject unknown overrides and overrides that remove mandatory gates. Arbitrary
step ordering, custom graph execution, workflow inheritance, and plugins wait for
observed needs. No external workflow engine dependency is required for validation.

## Durable run contract

The minimum persisted concepts are:

- **Task / SpecVersion:** stable requested outcome and immutable structured spec;
  a GitHub issue is an optional source, not the run identity.
- **WorkflowVersion:** immutable resolved step contracts and transition rules.
- **Run:** one authorized attempt pinned to spec/workflow/configuration/base
  revision, with limits, current state, and a dedicated workspace.
- **StepAttempt:** step identity, attempt number, input references, executor and
  agent-session identity where available, timestamps, outcome, and output refs.
- **Decision:** scoped operator request/resolution with run/version binding,
  single resolution, and expiry where specified. An agent cannot approve itself.
- **Artifact / Evidence:** inspectable file or record with identity, provenance,
  and revision binding where relevant. Preserve failure evidence as well as passes.
- **DeliveryAttempt:** intended external effect, authority, reconciliation status,
  and observed Git/GitHub identity.

Run states distinguish queued, running, awaiting operator, blocked, interrupted,
failed, cancelled, delivered with waiver, and review-ready. Step attempts
distinguish pending, running, awaiting operator, succeeded, failed, interrupted,
cancelled, and explicitly skipped. A skip records a reason and cannot satisfy a
required gate. Successful step completion and the next eligible transition
commit atomically with events.

Review-ready means the candidate revision satisfies the approved verification
and review contract and has a complete local simulated-delivery packet in V0.
That packet is not a real PR and does not prove GitHub behavior. Later real
delivery requires reconciled PR identity. Operator inspection/acceptance is
recorded separately; PR merge is not inferred from it.
Required-check waivers remain explicit named decisions and appear in the result;
a waived run is reported as delivered with waiver, not unqualified review-ready.

SQLite is authoritative for new managed runs; JSON remains an export/artifact
format. Legacy sessions remain inspectable with explicit legacy identity rather
than being presented as approved versioned runs.

## Execution and recovery

The daemon owns execution, not a connected CLI/TUI. One worker uses a dedicated
workspace and an appropriate OS-user/container boundary for unattended execution.
A worktree separates edits but is not a security sandbox. Keep delivery
credentials and controller sockets outside worker access. Verification commands
execute within the worker boundary too.

Enforce execution wall-clock and repair-attempt limits and cancellation of owned
processes. Accumulate elapsed executing time across attempts; waiting for a human
or external prerequisite is excluded, with no executor left running during that
wait. Persist attempt deadlines so a restart cannot reset the allowance; an
unreconciled running interval consumes time until the worker is confirmed stopped.
Report cost/usage when available; do not claim a hard spend cap without runtime
support. An unsupported required permission or limit fails preflight explicitly.
Runtime permission requests explicitly covered by the approved plan may be
granted only when the controller can match the specific requested operation to
that authority. Unknown, ambiguous, or out-of-scope requests pause for the
operator. Record both requests and decisions. There is no silent blanket grant;
`--yes` on a Conveyor command does not generally authorize later agent requests.

Persist action intent before external effects. Recovery reconciles processes,
workspaces, commits, branches, and PRs before repeating work. A daemon restart
must not create a second worker or duplicate delivery. Unknown outcomes remain
interrupted or awaiting operator until reconciled. Resume agent sessions only
when supported; otherwise preserve the attempt and require an explicit bounded
retry. Recovery is not guaranteed identical LLM output, automatic conversation
replay, or exactly-once execution across external systems.

## Run output

Plain text is the required first interface; JSON supports inspection/automation.
The TUI may render the same records without gaining separate workflow logic.
Show run/spec/workflow identities, overall state and current step, ordered step
attempts and durations, input/output artifact references, revisions, checks,
decisions, remaining limits, delivery link, and the next available operator action.
A fresh client must reconstruct this view without conversation history.

This is Temporal-like run visibility and durable progress, not a promise of
Temporal's complete execution/replay model. Keep bounded summaries and artifact
references; never persist credentials or full secret-bearing transcripts.

## Validation and deferred scope

The [example](example-task-contract.md) and delivery plan define the first proof:
an approved local pilot spec, a scripted failure repaired within the shared
allowance, simulated delivery, and a reviewable packet with revision-bound
evidence. Repeatable CI/controller evaluations use a fresh fixture repository
per run, deterministic scripted/fake agents, and simulated delivery for recovery
and idempotency tests. Containers are optional. Separate real-model Luna runs
measure variable outcomes, time, and cost. Never introduce artificial defects
into a real delivery. Always-on host/reboot and real GitHub delivery are later
optional validations, not V0 exit criteria.

Defer embedded planning chat, new browser UI, visual workflow editors, arbitrary
DAGs, parallel workers, multi-host scheduling, webhooks, notifications, knowledge
graphs, ontologies, and model routing. A few real runs and measured friction must
justify expansion. These requirements do not change existing command guarantees
until their replacement behavior is implemented and documented.
