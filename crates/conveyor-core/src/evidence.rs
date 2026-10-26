//! Durable, structured verification evidence.

use crate::verification::{Outcome, VerificationReport};

/// The revision and verification results attached to a local delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceReport {
    pub revision: String,
    pub verification: VerificationReport,
}

impl EvidenceReport {
    pub fn new(revision: impl Into<String>, verification: VerificationReport) -> Self {
        Self {
            revision: revision.into(),
            verification,
        }
    }

    /// Renders a stable JSON artifact suitable for local inspection or a PR body.
    pub fn to_json(&self) -> String {
        let checks = self
            .verification
            .results
            .iter()
            .map(|result| {
                let outcome = match result.outcome {
                    Outcome::Passed => "passed",
                    Outcome::Failed => "failed",
                };
                format!(
                    "{{\"name\":{},\"command\":{},\"required\":{},\"outcome\":{},\"exit_code\":{},\"duration_ms\":{}}}",
                    json_string(&result.name),
                    json_string(&result.command),
                    result.required,
                    json_string(outcome),
                    result.exit_code.map_or_else(|| "null".to_owned(), |code| code.to_string()),
                    result.duration_ms
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"revision\":{},\"passed_required_checks\":{},\"checks\":[{}]}}",
            json_string(&self.revision),
            self.verification.passes_required_checks(),
            checks
        )
    }
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}
