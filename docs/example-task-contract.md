# Example local pilot task and workflow

Planned V0 fixture, 2026-11-02. The Swan-like test repository and task below
have not been provisioned. This example describes the intended pilot, not
current Conveyor capability or an approved run. The [PRD](PRD.md) defines the
target contract; the [delivery plan](ROADMAP.md) defines implementation gates.

## Problem

The planned test repository contains a .NET API, a React/Vite frontend, and
Aspire orchestration. Add a backend health-details endpoint that reports the
running application's name and version. Keep this test task distinct from the
later real-world Swan League commissioner participation trial.

## In scope

- Add `GET /api/health/details` to the .NET API.
- Return the application name and version using values sourced from the app's
  existing configuration/build metadata, once those sources are confirmed.
- Add backend tests for the route, status, and returned name/version.
- Run only the focused test and repository checks proposed by the planning agent
  and explicitly included in the operator-approved plan.
- Produce a local simulated-delivery review packet.

## Out of scope

- Changes to React/Vite UI, Aspire orchestration, or unrelated services.
- A real branch push, GitHub issue mutation, PR creation/comment, merge, or deploy.
- Provisioning an always-on host, SSH/reboot testing, or proving real GitHub
  delivery behavior.
- Expanding the endpoint into a general diagnostics/metrics API.

## Acceptance criteria

- [ ] `GET /api/health/details` returns HTTP 200 and a documented JSON response
  containing the correct application name and version.
- [ ] Automated backend tests verify the route and both returned values.
- [ ] The focused tests and any additional approved repository checks pass
  for the final candidate revision; their exact commands are confirmed during
  planning before approval.
- [ ] The local review packet identifies the candidate revision and includes
  the diff, proposed PR title/summary, check outcomes, review findings, unresolved
  concerns, and simulated delivery identity/idempotency record.

## Affected components

The .NET API route/response and backend tests. Confirm exact project and file
locations after the test repository is provisioned. Do not edit React/Vite or
Aspire files for this task.

## Verification plan

During planning, the agent may inspect repository code and docs to propose the
focused test command and any required repository checks. Planning does not edit
code or execute checks. The operator approves the exact commands in the final
execution plan before the controller runs them. Record command, status,
duration, artifact reference, and candidate revision for every check. The
planned repository does not yet establish an exact .NET command, so do not
invent one here.

## Dependencies and risks

- The Swan-like .NET API/React/Vite/Aspire repository must be selected and
  provisioned; the API project and its existing name/version sources must be
  inspected before final approval.
- Define response property names, version source, and focused tests during
  planning. Any unresolved material behavior question blocks final approval.
- Implementation and review use separate role configuration entries, each
  selecting Luna through Codex ACP for this pilot. Confirm exact model IDs and
  adapter support before implementation. If unavailable, stop visibly for a
  decision; never silently substitute or incur unapproved provider cost.
- One repair attempt is shared across failed verification and in-scope review
  findings. A changed revision must return through verification and review.
- Allow 30 minutes of active execution, excluding human waits while no executor
  is running. Exhaustion yields a truthful failed/blocked outcome and preserves
  work and evidence.
- The pilot's delivery is simulated and local. It is not proof of a real GitHub
  branch or PR. A real GitHub smoke test requires separate explicit
  authorization in a dedicated repository.

## Open questions

- Which exact Swan-like test repository and base revision will be provisioned?
- What response field names and metadata source are appropriate for its API?
- Which focused .NET test command and repository checks should the operator
  approve after planning inspects the repository?
- Which exact Luna model identifiers and Codex ACP adapter support are available
  for both independently configured roles?

## Planned staged planning and approval

The desired entry point is `conveyor task create "Add health details endpoint"`.
The planning agent and operator work through outcome/scope, behavior/questions,
verification, and execution/authority in digestible stages. After each stage,
the agent summarizes the decisions and continues unless corrected. The agent may
read code and documentation but must not change code or run checks while
planning. It proposes the tests/checks for approval. After all material
questions are resolved, the controller displays the complete structured spec
and execution plan for one explicit final approval. Staged planning may initially
happen in an external agent; saving/resuming unfinished planning is future work.

One approval authorizes exactly the listed workflow: backend implementation,
the approved checks, review, and local simulated delivery. It grants no real
GitHub, merge, or deploy authority. Runtime permission requests may be granted
only when the controller matches the specific operation to that approved plan;
unknown, ambiguous, out-of-scope, or new-authority requests pause for the
operator. No blanket approval or general `--yes` bypass is implied.

After approval, a clean verification and review proceed automatically. Review
findings within scope may use the shared repair attempt and then require fresh
verification and review. Pause only for material scope/authority changes or a
blocker that cannot be resolved within the approved limits. A clean review does
not require another human gate. The operator inspects the resulting packet;
merge and deployment remain operator decisions.

## Resolved handoff (illustrative identifiers)

Task T1 is pinned to spec V1, workflow W1, configuration C1, and the selected
repository/base revision. W1 defines prepare, implement, verify, review, and
simulate-deliver, with one shared repair allowance. The two separately
configurable role selections both specify Luna through Codex ACP. The controller
records the exact supported model identifiers and resolved configuration before
approval; unsupported selections fail preflight rather than changing silently.

An operator approves T1/V1 + W1 + C1 for run R1: prepare a local workspace,
change only the .NET API and its tests, run the approved checks, review the
candidate, and create the local simulated packet. Limits are 30 minutes active
execution and one repair attempt shared by verification and review. Human waits
with no executor running do not consume active time. Changed scope, checks,
actions, configuration, or limits require a new spec/workflow/config approval.

## Expected run view (proposed output)

```text
Run R1 | spec T1/V1 | workflow W1 | config C1
State: simulated-delivery complete | step: simulate-deliver

prepare          #1 succeeded   base abc123, workspace <path>
implement        #1 succeeded   candidate def456, summary <artifact>
verify           #1 succeeded   approved checks passed on def456 <report>
review           #1 succeeded   Luna findings: none in scope <review packet>
simulate-deliver #1 succeeded   local packet <artifact>

Repair allowance remaining: 1
Active execution: 14m / 30m
Next: inspect the local review packet
```

If verification fails or review finds an in-scope defect, the shared repair
allowance can be used once. The changed candidate returns to verification and
review. A second repair need or exhausted time budget ends truthfully as failed
or blocked and preserves the diff and evidence.

## Required local and fixture demonstrations

- **Happy path:** approve the plan, complete implementation/checks/review, and
  produce a local packet. Do not claim a branch or PR exists.
- **Bounded repair:** inject a failure only in a fixture/fake-agent evaluation,
  preserve its report, repair once, then verify and review the new candidate.
  A second failure exhausts the shared allowance.
- **Permission matching:** grant an explicitly listed runtime operation only
  when the controller matches its specific action to the approved plan; pause
  on unknown or ambiguous requests and record request and decision.
- **Recovery and idempotency:** use a fresh fixture repository per run and
  deterministic scripted agents to test restart/recovery and repeated simulated
  delivery. Containers are optional. No live agent/provider is needed for CI.
- **Evidence invalidation:** change the candidate after verification; old
  results cannot satisfy checks or review for the new revision.
- **Attention and cancellation:** stop for new authority or unresolved blockers;
  cancellation stops owned work and preserves artifacts. Human waits consume no
  active time when no executor is running.
- **Waiver:** any authorized named check waiver remains visibly qualified and
  never becomes a passing check.

Later, an explicitly authorized real GitHub smoke test in a dedicated repository
may verify branch/PR reconciliation. Optional always-on-host work may test
SSH/disconnect, reboot, and backup/restore. Neither is required for this local
pilot. Never insert an artificial defect into a real delivery.
