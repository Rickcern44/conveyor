use std::fs;

use conveyor_core::session::{SessionRecord, SessionStore};
use conveyor_core::workflow::IssueState;

fn temporary_directory(name: &str) -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "conveyor-session-test-{name}-{}",
        std::process::id()
    ));
    let _ = fs::remove_dir_all(&path);
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn saves_and_loads_an_inspectable_session_record() {
    let directory = temporary_directory("save-load");
    let store = SessionStore::new(&directory);
    let mut record = SessionRecord::new(70, IssueState::InProgress, "feat/70-validate-issues");
    record.record_agent_outcome("fixture-agent --packet packet.md", Some(0));
    record.set_delivery_confirmation_policy(true, false);

    store.save(&record).unwrap();

    let saved = fs::read_to_string(directory.join(".conveyor/sessions/70.json")).unwrap();
    assert!(saved.contains("\"issue_number\": 70"));
    assert!(saved.contains("\"agent_exit_code\": 0"));
    assert!(saved.contains("\"require_push_confirmation\": true"));
    assert!(saved.contains("\"require_pr_confirmation\": false"));
    assert_eq!(store.load(70).unwrap(), Some(record));
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn lists_sessions_in_issue_number_order() {
    let directory = temporary_directory("list");
    let store = SessionStore::new(&directory);
    store
        .save(&SessionRecord::new(71, IssueState::Blocked, "feat/71"))
        .unwrap();
    store
        .save(&SessionRecord::new(70, IssueState::InProgress, "feat/70"))
        .unwrap();

    let sessions = store.list().unwrap();

    assert_eq!(
        sessions
            .into_iter()
            .map(|session| session.issue_number)
            .collect::<Vec<_>>(),
        vec![70, 71]
    );
    fs::remove_dir_all(directory).unwrap();
}

#[test]
fn records_a_blocker_owner_and_a_deliberate_resume_state() {
    let mut record = SessionRecord::new(70, IssueState::InProgress, "feat/70");

    record
        .block("waiting for API access", "platform-team")
        .unwrap();
    assert_eq!(record.state, IssueState::Blocked);
    assert_eq!(record.blocker.as_deref(), Some("waiting for API access"));
    assert_eq!(record.blocker_owner.as_deref(), Some("platform-team"));

    record.resume(IssueState::InProgress).unwrap();
    assert_eq!(record.state, IssueState::InProgress);
    assert_eq!(record.blocker, None);
    assert_eq!(record.blocker_owner, None);
}
