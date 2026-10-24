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
    let record = SessionRecord::new(70, IssueState::InProgress, "feat/70-validate-issues");

    store.save(&record).unwrap();

    let saved = fs::read_to_string(directory.join(".conveyor/sessions/70.json")).unwrap();
    assert!(saved.contains("\"issue_number\": 70"));
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
