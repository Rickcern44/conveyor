use conveyor_core::workflow::{IssueState, TransitionError};

#[test]
fn follows_the_happy_path_to_review() {
    let state = IssueState::NeedsSpec;
    let state = state.transition_to(IssueState::SpecReady).unwrap();
    let state = state.transition_to(IssueState::ReadyForBuild).unwrap();
    let state = state.transition_to(IssueState::InProgress).unwrap();

    assert_eq!(
        state.transition_to(IssueState::InReview).unwrap(),
        IssueState::InReview
    );
}

#[test]
fn rejects_skipping_a_required_state() {
    let error = IssueState::SpecReady
        .transition_to(IssueState::InProgress)
        .unwrap_err();

    assert_eq!(
        error,
        TransitionError::Disallowed {
            from: IssueState::SpecReady,
            to: IssueState::InProgress,
        }
    );
}

#[test]
fn permits_blocking_from_active_states_and_resuming_with_a_decision() {
    let blocked = IssueState::InProgress
        .transition_to(IssueState::Blocked)
        .unwrap();

    assert_eq!(
        blocked.transition_to(IssueState::ReadyForBuild).unwrap(),
        IssueState::ReadyForBuild
    );
}
