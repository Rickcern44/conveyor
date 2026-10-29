# Conveyor contributor guide

## Product boundary

Conveyor is a local-first Rust CLI. Read-only operations are the default;
implemented commands can create local configuration, run agents, change Git and
GitHub state, and deliver draft PRs with the applicable policy and operator
confirmation. A draft PR alone is not verified completion. Durable version-bound
approvals and unattended execution are planned, not current guarantees.
This product boundary applies to Conveyor's behavior. The coding assistant is
authorized to create and update GitHub issues for this repository when the user
requests issue work.

## Architecture

- Keep command parsing and terminal output in `crates/conveyor-cli`.
- Keep domain rules, configuration, issue-contract validation, and workflow
  transitions deterministic and testable in `crates/conveyor-core`.
- Add adapters only when a milestone needs them; do not couple core policy to
  GitHub, Git, or a specific agent runtime.
- Treat `.conveyor/` as local, inspectable execution state and never commit it.

## Development workflow

1. Write or update a failing unit or integration test before implementation.
2. Make the smallest production change that makes the test pass.
3. Run `cargo fmt --check`, `cargo clippy --workspace --all-targets -- -D warnings`,
   and `cargo test --workspace` before committing.
4. Use Conventional Commits (`feat:`, `fix:`, `test:`, `docs:`, `chore:`).

## Safety rules

- Read-only behavior is the default.
- Never add a side effect that bypasses explicit policy or operator consent.
- State-changing commands prompt for confirmation by default; `--yes` is the
  explicit non-interactive equivalent and is never a policy bypass.
- Avoid logging tokens, secrets, or full repository contents.
- Maintain clear human-readable errors and structured output paths for CLI
  commands.
