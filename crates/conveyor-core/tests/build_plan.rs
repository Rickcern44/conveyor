use conveyor_core::build_plan::{BuildInput, BuildPlanError, prepare};
use conveyor_core::workflow::IssueState;

#[test]
fn prepares_a_branch_only_for_a_ready_clean_and_current_issue() {
    let plan = prepare(BuildInput {
        issue_number: 70,
        state: IssueState::ReadyForBuild,
        worktree_clean: true,
        worktree_up_to_date: true,
        branch: "feat/70-handoff".to_owned(),
    })
    .unwrap();

    assert_eq!(plan.issue_number, 70);
    assert_eq!(plan.branch, "feat/70-handoff");
    assert_eq!(plan.next_state, IssueState::InProgress);
}

#[test]
fn rejects_builds_that_bypass_readiness_or_worktree_safety() {
    let state_error = prepare(BuildInput {
        issue_number: 70,
        state: IssueState::SpecReady,
        worktree_clean: true,
        worktree_up_to_date: true,
        branch: "feat/70-handoff".to_owned(),
    })
    .unwrap_err();
    assert_eq!(
        state_error,
        BuildPlanError::IssueNotReady(IssueState::SpecReady)
    );

    let worktree_error = prepare(BuildInput {
        issue_number: 70,
        state: IssueState::ReadyForBuild,
        worktree_clean: false,
        worktree_up_to_date: false,
        branch: "feat/70-handoff".to_owned(),
    })
    .unwrap_err();
    assert_eq!(worktree_error, BuildPlanError::DirtyWorktree);
}
