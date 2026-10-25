//! Configured local-check execution records.

use crate::config::Check;

/// Executes a configured command without exposing process details to policy.
pub trait CommandRunner {
    fn run(&self, command: &str) -> CommandResult;
}

/// The machine result returned by one command execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandResult {
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
}

impl CommandResult {
    pub fn completed(exit_code: i32, duration_ms: u128) -> Self {
        Self {
            exit_code: Some(exit_code),
            duration_ms,
        }
    }
}

/// The result of applying a configured check.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Outcome {
    Passed,
    Failed,
}

/// Evidence for one configured check execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CheckResult {
    pub name: String,
    pub command: String,
    pub required: bool,
    pub outcome: Outcome,
    pub exit_code: Option<i32>,
    pub duration_ms: u128,
}

/// The complete record created by `conveyor verify`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VerificationReport {
    pub results: Vec<CheckResult>,
}

impl VerificationReport {
    pub fn passes_required_checks(&self) -> bool {
        self.results
            .iter()
            .all(|result| !result.required || result.outcome == Outcome::Passed)
    }
}

/// Runs every configured check and records both required and optional results.
pub fn verify(runner: &impl CommandRunner, checks: &[Check]) -> VerificationReport {
    VerificationReport {
        results: checks
            .iter()
            .map(|check| {
                let execution = runner.run(&check.command);
                CheckResult {
                    name: check.name.clone(),
                    command: check.command.clone(),
                    required: check.required,
                    outcome: if execution.exit_code == Some(0) {
                        Outcome::Passed
                    } else {
                        Outcome::Failed
                    },
                    exit_code: execution.exit_code,
                    duration_ms: execution.duration_ms,
                }
            })
            .collect(),
    }
}
