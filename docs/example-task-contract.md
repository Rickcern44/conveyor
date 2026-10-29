# Example task contract (illustrative, not an approved pilot)

This uses the canonical issue sections and illustrates the planned M0 handoff;
current Conveyor does not yet enforce version-bound approval or run limits.

## Problem

A command's help text omits an existing `--verbose` option, making it difficult
for users to discover diagnostic output.

## In scope

Add the option to the command's help text and test that it appears. Preserve
existing option behavior.

## Out of scope

Changing diagnostic behavior, adding options, deployment, or merging a PR.

## Acceptance criteria

- Help includes `--verbose` and describes its existing behavior accurately.
- The help test and the repository's required checks pass on the final revision.

## Affected components

CLI help and its tests; inspect the current parser before editing.

## Verification plan

Run `cargo fmt --check`, `cargo clippy --locked --workspace --all-targets -- -D warnings`,
and `cargo test --locked --workspace`. Record commands, results and final commit;
record skips or named waivers separately. A waiver does not turn failure into a pass.

## Dependencies and risks

The existing behavior must be confirmed before describing it. No provider or
production credentials are needed for the test.

## Open questions

If the current behavior differs from its documentation, stop and ask the
operator whether to revise scope; do not silently change behavior.

## Approval and limits (example values)

Task T1 version V1 is linked to this issue. The operator approves V1 and a
specific configuration snapshot for one run R1: edit the described files, run
the listed checks, create a branch, push it, and open a draft PR. No merge,
deploy, issue edits or other actions are authorized. Limit R1 to 30 minutes and
one repair attempt; these values are illustrative and require enforceable
runtime support. Changed scope, snapshot or limits require new approval.

## Example progression and outcomes

R1 starts only after that approval. If behavior is unclear, it enters awaiting
operator with a specific decision; an answer within the approved scope may
resume R1 if supported and approval remains valid. A broader change creates a
new version and approval. An unavailable dependency makes R1 blocked until it
is resolved. A failed check after the repair limit makes R1 failed; cancellation
makes R1 cancelled and preserves its work. Either terminal outcome requires a
new authorized run to try again. If checks pass on the produced revision and
the authorized draft PR and evidence are recorded, R1 is review-ready. The
operator's acceptance of the agreed outcome makes it completed; the draft PR
alone does not.
