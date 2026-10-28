use conveyor_core::control_plane::ControlPlaneStore;
use conveyor_core::event::{AgentEvent, AgentEventKind, EventContext};

#[test]
fn opens_a_repository_local_database_and_applies_the_initial_schema() {
    let root = std::env::temp_dir().join(format!("conveyor-control-plane-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let store = ControlPlaneStore::open(&root).unwrap();

    assert_eq!(store.path(), root.join(".conveyor/conveyor.db"));
    assert_eq!(store.schema_version().unwrap(), 1);
    assert!(store.has_table("agent_events").unwrap());
    assert!(store.has_table("pending_decisions").unwrap());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn records_normalized_agent_events_idempotently() {
    let root = std::env::temp_dir().join(format!("conveyor-control-events-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ControlPlaneStore::open(&root).unwrap();
    let event = AgentEvent::new(
        EventContext::new("event-1", 4, Some(70), Some("acp-1".to_owned())),
        AgentEventKind::PlanUpdated { steps: 3 },
    );

    store.record_event(&event).unwrap();
    store.record_event(&event).unwrap();

    assert_eq!(store.event_count().unwrap(), 1);
    assert!(
        store
            .export_json()
            .unwrap()
            .contains("\"event_id\":\"event-1\"")
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn persists_and_resolves_a_pending_decision_once() {
    let root =
        std::env::temp_dir().join(format!("conveyor-control-decisions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let store = ControlPlaneStore::open(&root).unwrap();
    let decision = store
        .create_pending_decision("permission", "Run git push", 100, None)
        .unwrap();

    assert_eq!(store.pending_decisions().unwrap().len(), 1);
    assert!(
        store
            .resolve_decision(decision.id, "allow_once", 150)
            .unwrap()
    );
    assert!(!store.resolve_decision(decision.id, "denied", 151).unwrap());
    assert!(store.pending_decisions().unwrap().is_empty());
    std::fs::remove_dir_all(root).unwrap();
}
