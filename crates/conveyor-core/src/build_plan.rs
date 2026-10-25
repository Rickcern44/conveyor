//! Safety preflight for the state-changing build command.

use std::fmt;

use crate::workflow::IssueState;

/// Facts observed before Conveyor is allowed to begin a build session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildInput {
    pub issue_number: u64,
    pub state: IssueState,
    pub worktree_clean: bool,
    pub worktree_up_to_date: bool,
    pub branch: String,
}

/// The allowed state-changing work that a CLI adapter may execute after consent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildPlan {
    pub issue_number: u64,
    pub branch: String,
    pub next_state: IssueState,
}

/// A build precondition that prevents all branch and GitHub mutations.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BuildPlanError {
    IssueNotReady(IssueState),
    DirtyWorktree,
    WorktreeOutOfDate,
}

impl fmt::Display for BuildPlanError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::IssueNotReady(state) => {
                write!(formatter, "issue is {state:?}, not ready-for-build")
            }
            Self::DirtyWorktree => write!(formatter, "worktree has uncommitted changes"),
            Self::WorktreeOutOfDate => write!(formatter, "worktree is not up to date"),
        }
    }
}

impl std::error::Error for BuildPlanError {}

/// Validates a build request before any adapter receives mutation authority.
pub fn prepare(input: BuildInput) -> Result<BuildPlan, BuildPlanError> {
    if input.state != IssueState::ReadyForBuild {
        return Err(BuildPlanError::IssueNotReady(input.state));
    }
    if !input.worktree_clean {
        return Err(BuildPlanError::DirtyWorktree);
    }
    if !input.worktree_up_to_date {
        return Err(BuildPlanError::WorktreeOutOfDate);
    }
    Ok(BuildPlan {
        issue_number: input.issue_number,
        branch: input.branch,
        next_state: IssueState::InProgress,
    })
}
