use conveyor_core::control_plane::ControlPlaneStore;
use conveyor_core::event::{AgentEvent, AgentEventKind, EventContext};

#[test]
fn opens_a_repository_local_database_and_applies_the_initial_schema() {
    let root = std::env::temp_dir().join(format!("conveyor-control-plane-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();

    let store = ControlPlaneStore::open(&root).unwrap();

    assert_eq!(store.path(), root.join(".conveyor/conveyor.db"));
    assert_eq!(store.schema_version().unwrap(), 2);
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

#[test]
fn versioned_contracts_survive_reopen_and_cannot_be_replaced() {
    use conveyor_core::control_plane::{SpecVersion, WorkflowVersion};
    let root = std::env::temp_dir().join(format!("conveyor-versions-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let spec = SpecVersion {
        task_id: "task-1".into(),
        version: 1,
        contract: "approved spec content".into(),
    };
    let workflow = WorkflowVersion {
        workflow_id: "pilot".into(),
        version: 1,
        definition: "resolved workflow content".into(),
    };
    {
        let store = ControlPlaneStore::open(&root).unwrap();
        store.insert_spec_version(&spec).unwrap();
        let mut invalid = spec.clone();
        invalid.version = 0;
        assert!(store.insert_spec_version(&invalid).is_err());
        invalid.version = 2;
        invalid.task_id = " ".into();
        assert!(store.insert_spec_version(&invalid).is_err());
        store.insert_workflow_version(&workflow).unwrap();
        let mut next = spec.clone();
        next.version = 2;
        next.contract = "new version".into();
        store.insert_spec_version(&next).unwrap();
        let mut changed = spec.clone();
        changed.contract = "changed".into();
        assert!(store.insert_spec_version(&changed).is_err());
        let mut changed_workflow = workflow.clone();
        changed_workflow.definition = "changed".into();
        assert!(store.insert_workflow_version(&changed_workflow).is_err());
    }
    let store = ControlPlaneStore::open(&root).unwrap();
    assert_eq!(store.spec_version("task-1", 1).unwrap(), Some(spec));
    assert_eq!(store.workflow_version("pilot", 1).unwrap(), Some(workflow));
    assert_eq!(
        store.spec_version("task-1", 2).unwrap().unwrap().contract,
        "new version"
    );
    assert_eq!(store.spec_version("task-1", 3).unwrap(), None);
    drop(store);
    let connection = rusqlite::Connection::open(root.join(".conveyor/conveyor.db")).unwrap();
    assert!(
        connection
            .execute(
                "UPDATE spec_versions SET contract = 'changed' WHERE task_id = 'task-1'",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "INSERT OR REPLACE INTO spec_versions VALUES ('task-1', 1, 'replaced')",
                []
            )
            .is_err()
    );
    assert!(
        connection
            .execute(
                "DELETE FROM workflow_versions WHERE workflow_id = 'pilot'",
                []
            )
            .is_err()
    );
    drop(connection);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn migration_upgrades_existing_v1_database_without_losing_events() {
    let root = std::env::temp_dir().join(format!("conveyor-migrate-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(root.join(".conveyor")).unwrap();
    let connection = rusqlite::Connection::open(root.join(".conveyor/conveyor.db")).unwrap();
    connection.execute_batch("CREATE TABLE schema_migrations (version INTEGER PRIMARY KEY, applied_at_unix_seconds INTEGER NOT NULL); INSERT INTO schema_migrations VALUES (1, 0); CREATE TABLE agent_events (event_id TEXT PRIMARY KEY, issue_number INTEGER, agent_session_id TEXT, schema_version INTEGER NOT NULL, category TEXT NOT NULL, summary TEXT NOT NULL, timestamp_unix_millis INTEGER NOT NULL); INSERT INTO agent_events VALUES ('legacy', NULL, NULL, 1, 'plan', 'legacy', 0);").unwrap();
    drop(connection);
    let store = ControlPlaneStore::open(&root).unwrap();
    assert_eq!(store.schema_version().unwrap(), 2);
    assert_eq!(store.event_count().unwrap(), 1);
    assert!(store.has_table("spec_versions").unwrap());
    assert!(store.has_table("workflow_versions").unwrap());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
