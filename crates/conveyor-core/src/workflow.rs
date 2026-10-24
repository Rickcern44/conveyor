//! Issue workflow states and the transitions that Conveyor permits.

use std::fmt;

/// The lifecycle state of an issue managed by Conveyor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum IssueState {
    NeedsSpec,
    SpecReady,
    ReadyForBuild,
    InProgress,
    InReview,
    Done,
    Blocked,
}

impl IssueState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NeedsSpec => "needs-spec",
            Self::SpecReady => "spec-ready",
            Self::ReadyForBuild => "ready-for-build",
            Self::InProgress => "in-progress",
            Self::InReview => "in-review",
            Self::Done => "done",
            Self::Blocked => "blocked",
        }
    }

    pub fn parse(value: &str) -> Option<Self> {
        match value {
            "needs-spec" => Some(Self::NeedsSpec),
            "spec-ready" => Some(Self::SpecReady),
            "ready-for-build" => Some(Self::ReadyForBuild),
            "in-progress" => Some(Self::InProgress),
            "in-review" => Some(Self::InReview),
            "done" => Some(Self::Done),
            "blocked" => Some(Self::Blocked),
            _ => None,
        }
    }

    /// Moves to `next` when the workflow explicitly permits that transition.
    pub fn transition_to(self, next: Self) -> Result<Self, TransitionError> {
        if self.allows(next) {
            Ok(next)
        } else {
            Err(TransitionError::Disallowed {
                from: self,
                to: next,
            })
        }
    }

    fn allows(self, next: Self) -> bool {
        matches!(
            (self, next),
            (Self::NeedsSpec, Self::SpecReady)
                | (Self::NeedsSpec, Self::Blocked)
                | (Self::SpecReady, Self::ReadyForBuild)
                | (Self::SpecReady, Self::Blocked)
                | (Self::ReadyForBuild, Self::InProgress)
                | (Self::ReadyForBuild, Self::Blocked)
                | (Self::InProgress, Self::InReview)
                | (Self::InProgress, Self::Blocked)
                | (Self::InReview, Self::Done)
                | (Self::InReview, Self::Blocked)
                | (Self::Blocked, Self::NeedsSpec)
                | (Self::Blocked, Self::SpecReady)
                | (Self::Blocked, Self::ReadyForBuild)
                | (Self::Blocked, Self::InProgress)
                | (Self::Blocked, Self::InReview)
        )
    }
}

/// Describes why a workflow transition cannot be applied.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TransitionError {
    Disallowed { from: IssueState, to: IssueState },
}

impl fmt::Display for TransitionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Disallowed { from, to } => {
                write!(
                    formatter,
                    "transition from {from:?} to {to:?} is not allowed"
                )
            }
        }
    }
}

impl std::error::Error for TransitionError {}
