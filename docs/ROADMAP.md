# Conveyor delivery plan

Status: proposed narrow validation, 2026-11-02. Supersedes the earlier M0–M7
sequence. The [PRD](PRD.md) is the target contract; README describes current
behavior. These slices are planned, not implemented guarantees.

## First outcome

Run the first V0 pilot locally on the operator's Mac against a planned,
Swan-like fixture repository (.NET API, React/Vite, Aspire). Its backend-only
task is `GET /api/health/details` returning the application name and version,
covered by tests. This repository and task have not been provisioned. The pilot
uses a local simulated-delivery packet, not a real GitHub branch or PR. A later
real-world trial uses Swan League commissioner participation work and remains
distinct from this test pilot. An always-on host, SSH/reboot survival, and real
draft-PR smoke tests are optional future validations, not V0 prerequisites.

The desired entry point is `conveyor task create "Task to do"`. A human-facing
planning agent stages outcome/scope, behavior/questions, verification, and
execution/authority in digestible chunks, summarizes each stage, and continues
unless corrected. It proposes focused checks but does not change code or run
checks during planning. One final explicit approval binds the complete workflow.
The interaction is planned; initial development may use an external planning
agent before embedding this flow. Saving/resuming planning conversations is
future work.

## Reuse and change

| Existing foundation | Treatment in this validation |
| --- | --- |
| Issue validation and task/review packets | Reuse as spec intake and step inputs; add immutable spec identity |
| Workflow labels and JSON sessions | Preserve legacy commands; new run state is authoritative in SQLite, labels are projections |
| SQLite events and decisions | Extend with runs, step attempts, artifact refs, version bindings, and decision enforcement |
| ACP and direct adapters | Choose one supported runtime for the proof; retain interfaces without requiring new parity work |
| CLI-owned agent execution | Extract shared application operations and move managed runs into daemon supervision |
| Verification, Git/GitHub delivery | Reuse adapters behind revision-bound gates and reconciliation records |
| TUI and daemon socket | Add only run inspection/control needed by the proof; plain-text CLI first |

New managed runs use the new contract. Existing build/verify/review paths retain
their documented behavior until deliberately migrated; do not silently confer
new guarantees on them. Migration preserves legacy inspection/export without
requiring a fabricated historical approval.

## V0 — freeze the local simulated pilot contract

Use the small Swan-like test repository and backend-only health-details task
described above. Confirm/provision them before implementation; they are not
assumed to exist. The task returns application name and version from
`GET /api/health/details` and includes tests. Keep React/Vite and Aspire changes
out of scope unless the operator approves a revised spec. Swan League's
commissioner participation work is a later real-world trial, not the fixture task.

Use Codex through ACP with Luna for both implementation and review in separately
configurable role selections. Confirm exact model identifiers and adapter
support before implementing the pilot. Missing support is a visible preflight
failure; never silently switch provider/model or incur unapproved cost. OpenCode
is a possible later ACP provider.

Record the base revision, accepted behavior/response contract, approved test
commands, review criteria, local execution authority, and simulated delivery
outputs. One repair attempt is shared across failed checks and in-scope review
findings. Allow 30 minutes of active execution, excluding human waits when no
executor is running. A changed revision returns through verification and review.
Stop truthfully and preserve artifacts when the repair or time budget is spent.

Simulated delivery creates a local packet containing proposed PR title/summary,
diff, base/target and candidate revisions, check results, review findings,
unresolved concerns, and a simulated delivery identity/idempotency record. It
must not be reported as evidence of real GitHub behavior. A clean review does
not require an additional human approval; the operator reviews the completed
packet afterward, while merge/deploy remain operator decisions. Runtime
permission requests are granted only if the controller matches the specific
operation to the approved plan; unknown or ambiguous requests pause.

Exit: the example contract and resolved plan let an operator predict every
transition, authority, limit, pause condition, and artifact. The test repository,
task, model IDs, and ACP support are confirmed before execution; unresolved
choices are visible rather than delegated implicitly. V0 does not require a
remote host, disconnection survival, a live GitHub mutation, or a real PR.

## V1 — persist and inspect a run with step attempts

Implement the minimum domain model and SQLite migrations for immutable spec and
resolved workflow versions, run/configuration approval, step attempts, artifacts,
decisions, and ordered events. Keep transitions deterministic in conveyor-core.
Commit state changes and events transactionally. Validate overrides before
approval; pin the resolved workflow so edits only affect future runs.

Use a fresh fixture repository per run and deterministic scripted/fake agents to
advance through the fixed workflow, fail a verification or review attempt,
consume the shared repair allowance, simulate delivery, and reconstruct the
result after reopening the store. Define read-only run
list/detail/event/result operations with plain text and JSON output; final CLI
spelling is an implementation choice. Containers are optional.

Exit: a fresh CLI renders every attempt and its artifacts without the original
process. Failed checks cannot advance the run, exhausted repairs stop it, and
spec/workflow changes cannot mutate an approved run. Reject stale/duplicate
operator decisions. Legacy sessions remain distinguishable and inspectable.

## V2 — execute one durable background worker

Move managed execution into conveyord through shared application services and a
minimal versioned local API: submit/start, inspect, decide, cancel, and explicitly
retry interrupted work. Keep the Unix socket local. Reuse existing packet and
agent adapters for the selected Codex ACP/Luna role configurations. Run the
initial worker locally on the Mac; do not make an always-on host a prerequisite.

Prepare a dedicated workspace, record base/environment, and apply the chosen
local worker boundary. Enforce one active worker, the approved 30-minute active
execution limit and shared repair allowance, owned process cancellation, and
durable permission requests. Match a runtime permission request to the exact
approved operation before granting; unknown/ambiguous requests wait for the
operator. Keep controller/delivery credentials out of the worker. Artifact
retention must preserve failed work.

On restart, reconcile owned processes before dispatching another attempt. Resume
only supported sessions; otherwise record interruption and expose a deliberate
retry. A pending decision must survive disconnect/restart without silent approval.

Exit: start from CLI, disconnect the client, reconnect, answer a pending decision,
and observe the same run. Kill the daemon and verify no duplicate agent starts.
Cancel/timeout the worker and confirm owned processes stop and work is preserved.
Use fake executors for fault injection before running the supported real agent.

## V3 — gate results and simulate delivery locally

Connect the step executors to existing verification and review-packet adapters.
Verify the candidate revision within the worker boundary. Map acceptance
criteria to evidence and resolve any required subjective decisions during
planning, before final approval. Review-agent findings within scope may consume
the shared repair allowance; a material scope change or new authority pauses for
operator approval. Changed revisions invalidate later evidence. Required-check
waivers produce a distinct qualified outcome.

Persist simulated delivery intent and idempotency identity; generate the local
review packet with proposed PR title/summary, diff, base/target and candidate
revisions, checks, review findings, unresolved concerns, and delivery identity.
Render the complete run timeline, attempts, decisions, artifact paths, and next
action in text. TUI reuse is optional and must use the same operations.

Exit: an approved local spec reaches a simulated-delivery packet with passing
evidence for its candidate revision. A required failure prevents review-ready;
a fixture failure proves the one-repair route across verification and review.
Repeated simulated delivery is idempotent. Exhausted limits yield a useful
failed/blocked result. No real GitHub mutation is needed to claim this gate.

## V4 — optional real-delivery and host validation

After the local workflow is useful, the operator may authorize a real GitHub
smoke test in a dedicated repository and/or select an always-on host for
deployment validation. Neither is required for V0 or local simulated delivery.
If a host is selected, document install/update, runtime versions, authentication,
local socket access, logs, health checks, artifact paths, and a consistent
SQLite-plus-artifacts backup/restore procedure. Use SSH only for host validation.
A remote API, laptop controller, and new UI are unnecessary.

For an authorized real-delivery smoke test, record actual branch/PR identity and
reconciliation evidence. Do not run it on every pilot or imply simulated delivery
proves GitHub behavior. For optional host validation, record these demonstrations:

1. Approve a real run over SSH, close the laptop, and return to a result or a
   durable attention request. The box remains powered and owns execution.
2. Restart the daemon and reboot the host during execution. Reconcile existing
   effects, preserve history, and expose interrupted work honestly; automatic
   agent resumption is not required.
3. Answer a pending decision after reconnect; reject an expired decision.
4. Repeat cancellation, timeout, failed verification, and interrupted-delivery
   drills against the installed environment.
5. Restore a consistent backup into a separate location and inspect the run and
   referenced artifacts without accidentally executing restored work.

Exit: if undertaken, the selected validation has recorded outcomes and no
unexplained active processes, missing required artifacts, or duplicate
deliveries. This is optional technical evidence, not a prerequisite for the
local pilot and not proof of general productivity or a production platform.

## V5 — decide whether further composition earns its cost

Use fresh fixture repositories with deterministic fake agents and simulated
delivery for repeatable CI and controller recovery/idempotency evaluations.
Separately run real-model Luna evaluations to measure variable outcomes, elapsed
time, operator time/interventions, repair/review findings, and available usage
cost. Define baseline and success threshold before reviewing outcomes. Containers
are optional. Never insert an artificial defect into a real delivery.

After the test-repository pilot, a separate real-world trial may use Swan League
commissioner participation work. Do not conflate that task or its outcomes with
the Swan-like test fixture. Record failures as well as successes; a small number
of runs is an initial signal, not a statistically reliable claim.

Exit: document whether to continue, simplify, or adopt another tool. Expand only
when runs expose a repeated unmet need. Keep learning outcomes separate from
productivity results. The next workflow variation should come from a real task;
prove that a validated override suffices before adding arbitrary steps or graphs.

## Implementation discipline and immediate next slice

For production changes, write failing behavior tests first, implement the smallest
slice, and run fmt, workspace Clippy with warnings denied, and workspace tests as
required by AGENTS.md. Test recovery with fake agents and local Git/GitHub fixtures,
then repeat critical demonstrations with the selected runtime and host. Avoid
secret-bearing transcripts. `--yes` is never a policy bypass or blanket permission
for later agent actions. Current documentation work changes no runtime behavior.

Next: confirm/provision V0's local test repository and task, confirm model IDs and
ACP support, then implement V1's spec/workflow/run/step-attempt records,
transition rules, and text inspection using deterministic fake agents. Do not
start with a generic engine, embedded planner, or TUI redesign. Always-on-host,
SSH/reboot, and real-GitHub validation remain later optional gates; claim those
guarantees only after their own demonstrations pass.
