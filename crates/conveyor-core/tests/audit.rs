use std::fs;

use conveyor_core::audit::{AuditEvent, AuditLog};

#[test]
fn appends_inspectable_events_for_one_issue() {
    let root = std::env::temp_dir().join(format!("conveyor-audit-{}", std::process::id()));
    let _ = fs::remove_dir_all(&root);
    fs::create_dir_all(&root).unwrap();

    AuditLog::new(&root)
        .append(
            70,
            &AuditEvent::new("create_branch", "feat/70-build", "operator", 1_700_000_000),
        )
        .unwrap();

    let content = fs::read_to_string(root.join(".conveyor/audit/70.jsonl")).unwrap();
    assert!(content.contains("\"action\":\"create_branch\""));
    assert!(content.contains("\"target\":\"feat/70-build\""));
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn escapes_control_characters_without_breaking_json_lines() {
    let directory =
        std::env::temp_dir().join(format!("conveyor-audit-escaped-{}", std::process::id()));
    let _ = fs::remove_dir_all(&directory);
    fs::create_dir_all(&directory).unwrap();
    let log = AuditLog::new(&directory);

    log.append(
        70,
        &AuditEvent::new("record_blocker", "waiting\nfor access", "operator", 1),
    )
    .unwrap();

    let content = fs::read_to_string(directory.join(".conveyor/audit/70.jsonl")).unwrap();
    assert_eq!(content.lines().count(), 1);
    assert!(content.contains("waiting\\nfor access"));
    fs::remove_dir_all(directory).unwrap();
}
