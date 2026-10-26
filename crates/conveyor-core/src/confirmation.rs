//! Explicit operator authorization for state-changing commands.

/// Requests a one-time decision from an interactive operator.
pub trait ConfirmationPrompt {
    fn confirm(&self, summary: &str) -> bool;
}

/// The authorization result for the action described to the operator.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Decision {
    Approved,
    Declined,
}

/// Resolves confirmation from an explicit `--yes` or the interactive prompt.
///
/// `--yes` is intentional command-local authorization; policy must be checked
/// by the caller before this function is used.
pub fn authorize(explicit_yes: bool, prompt: &impl ConfirmationPrompt, summary: &str) -> Decision {
    if explicit_yes || prompt.confirm(summary) {
        Decision::Approved
    } else {
        Decision::Declined
    }
}
