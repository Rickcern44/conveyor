use conveyor_core::event::{AgentEvent, AgentEventKind, EVENT_SCHEMA_VERSION, EventContext};

#[test]
fn jsonl_event_has_a_versioned_identity_and_no_unbounded_transcript() {
    let event = AgentEvent::new(
        EventContext::new(
            "event-7",
            1_700_000_000_123,
            Some(70),
            Some("session-abc".to_owned()),
        ),
        AgentEventKind::ToolCompleted {
            title: "Run checks".to_owned(),
            outcome: "passed".to_owned(),
        },
    );

    let json = event.to_jsonl();

    assert!(json.contains(&format!("\"schema_version\":{EVENT_SCHEMA_VERSION}")));
    assert!(json.contains("\"event_id\":\"event-7\""));
    assert!(json.contains("\"issue_number\":70"));
    assert!(json.contains("\"session_id\":\"session-abc\""));
    assert!(json.contains("\"category\":\"tool.completed\""));
    assert!(json.contains("\"summary\":\"Run checks: passed\""));
}

#[test]
fn event_log_persists_only_normalized_jsonl() {
    let root = std::env::temp_dir().join(format!("conveyor-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let event = AgentEvent::new(
        EventContext::new("event-9", 2, Some(70), None),
        AgentEventKind::PermissionDecision {
            decision: "denied".to_owned(),
        },
    );
    conveyor_core::event::EventLog::new(&root)
        .append(&event)
        .unwrap();
    let content = std::fs::read_to_string(root.join(".conveyor/events/70.jsonl")).unwrap();
    assert!(content.contains("permission.decision"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn permission_decision_is_auditable_without_tool_output() {
    let event = AgentEvent::new(
        EventContext::new("event-8", 1, Some(70), None),
        AgentEventKind::PermissionDecision {
            decision: "allow_once".to_owned(),
        },
    );

    assert_eq!(
        event.to_jsonl(),
        "{\"schema_version\":1,\"event_id\":\"event-8\",\"timestamp_unix_millis\":1,\"issue_number\":70,\"session_id\":null,\"category\":\"permission.decision\",\"summary\":\"allow_once\"}"
    );
}
