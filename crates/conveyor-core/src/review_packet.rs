//! Portable review context assembled after local verification.

use crate::evidence::EvidenceReport;
use crate::issue_contract::IssueContract;
use crate::verification::Outcome;

/// Contract, diff summary, and verification evidence for a reviewer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewPacket {
    pub issue_number: u64,
    pub contract: IssueContract,
    pub diff_summary: String,
    pub evidence: EvidenceReport,
}

impl ReviewPacket {
    /// Renders an inspectable Markdown review brief.
    pub fn to_markdown(&self) -> String {
        let checks = self
            .evidence
            .verification
            .results
            .iter()
            .map(|result| {
                let outcome = match result.outcome {
                    Outcome::Passed => "passed",
                    Outcome::Failed => "failed",
                };
                format!("- {}: {outcome} ({}ms)", result.name, result.duration_ms)
            })
            .collect::<Vec<_>>()
            .join("\n");
        format!(
            "# Conveyor review packet: issue #{}\n\n## Contract\n\n### Problem\n\n{}\n\n### Acceptance criteria\n\n{}\n\n### In scope\n\n{}\n\n### Out of scope\n\n{}\n\n## Diff summary\n\n{}\n\n## Verification evidence\n\nrevision: `{}`\n\n{}\n",
            self.issue_number,
            self.contract.problem,
            self.contract.acceptance_criteria,
            self.contract.in_scope,
            self.contract.out_of_scope,
            self.diff_summary,
            self.evidence.revision,
            checks,
        )
    }
}
