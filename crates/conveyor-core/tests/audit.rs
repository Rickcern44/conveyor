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
