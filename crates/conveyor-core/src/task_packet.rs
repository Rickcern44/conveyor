//! Portable, bounded briefs for planner, implementer, and reviewer adapters.

use crate::config::Check;
use crate::issue_contract::IssueContract;

/// The authority-limited role for a task-packet recipient.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AgentRole {
    Planner,
    Implementer,
    Reviewer,
}

impl AgentRole {
    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "planner" => Some(Self::Planner),
            "implementer" => Some(Self::Implementer),
            "reviewer" => Some(Self::Reviewer),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Planner => "planner",
            Self::Implementer => "implementer",
            Self::Reviewer => "reviewer",
        }
    }
}

/// A self-contained implementation or review brief.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskPacket {
    pub issue_number: u64,
    pub role: AgentRole,
    pub branch: String,
    pub commit_message: String,
    pub contract: IssueContract,
    pub repository_guidance: String,
    pub checks: Vec<Check>,
}

impl TaskPacket {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        issue_number: u64,
        role: AgentRole,
        branch: impl Into<String>,
        commit_message: impl Into<String>,
        contract: IssueContract,
        repository_guidance: impl Into<String>,
        checks: Vec<Check>,
    ) -> Self {
        Self {
            issue_number,
            role,
            branch: branch.into(),
            commit_message: commit_message.into(),
            contract,
            repository_guidance: repository_guidance.into(),
            checks,
        }
    }

    /// Renders the packet as a portable Markdown artifact.
    pub fn to_markdown(&self) -> String {
        let checks = if self.checks.is_empty() {
            "- No configured checks.".to_owned()
        } else {
            self.checks
                .iter()
                .map(|check| {
                    let required = if check.required {
                        "required"
                    } else {
                        "optional"
                    };
                    format!("- {required}: {} — `{}`", check.name, check.command)
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        format!(
            "# Conveyor task packet: issue #{}\n\nRole: {}\n\nBranch: `{}`\n\nCommit convention: `{}`\n\n## Authority boundary\n\nImplement the scoped change and report results. Do not push, open or merge pull requests, deploy, or change GitHub issue state; Conveyor and the operator retain those actions.\n\n## Repository guidance\n\n{}\n\n## Problem\n\n{}\n\n## In scope\n\n{}\n\n## Out of scope\n\n{}\n\n## Acceptance criteria\n\n{}\n\n## Affected components\n\n{}\n\n## Verification plan\n\n{}\n\n## Dependencies and risks\n\n{}\n\n## Open questions\n\n{}\n\n## Required checks\n\n{}\n",
            self.issue_number,
            self.role.as_str(),
            self.branch,
            self.commit_message,
            self.repository_guidance,
            self.contract.problem,
            self.contract.in_scope,
            self.contract.out_of_scope,
            self.contract.acceptance_criteria,
            self.contract.affected_components,
            self.contract.verification_plan,
            self.contract.dependencies_risks,
            self.contract.open_questions,
            checks,
        )
    }
}
