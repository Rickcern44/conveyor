use conveyor_core::confirmation::{ConfirmationPrompt, Decision, authorize};

struct ApprovingPrompt;

impl ConfirmationPrompt for ApprovingPrompt {
    fn confirm(&self, _summary: &str) -> bool {
        true
    }
}

struct DecliningPrompt;

impl ConfirmationPrompt for DecliningPrompt {
    fn confirm(&self, _summary: &str) -> bool {
        false
    }
}

#[test]
fn explicit_yes_authorizes_non_interactive_work() {
    assert_eq!(
        authorize(true, &DecliningPrompt, "create branch"),
        Decision::Approved
    );
}

#[test]
fn interactive_prompt_controls_unattended_work_without_yes() {
    assert_eq!(
        authorize(false, &ApprovingPrompt, "create branch"),
        Decision::Approved
    );
    assert_eq!(
        authorize(false, &DecliningPrompt, "create branch"),
        Decision::Declined
    );
}
