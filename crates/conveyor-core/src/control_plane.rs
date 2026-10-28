//! Repository-local SQLite control-plane storage.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::event::AgentEvent;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PendingDecision {
    pub id: i64,
    pub kind: String,
    pub summary: String,
    pub expires_at_unix_seconds: Option<i64>,
}

/// Opens and migrates `.conveyor/conveyor.db` for one repository.
pub struct ControlPlaneStore {
    path: PathBuf,
    connection: Connection,
}

impl ControlPlaneStore {
    pub fn open(repository_root: impl AsRef<Path>) -> Result<Self, ControlPlaneError> {
        let path = repository_root.as_ref().join(".conveyor/conveyor.db");
        let parent = path.parent().expect("database path has a parent");
        std::fs::create_dir_all(parent).map_err(ControlPlaneError::Io)?;
        let connection = Connection::open(&path).map_err(ControlPlaneError::Sql)?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON; PRAGMA journal_mode = WAL;")
            .map_err(ControlPlaneError::Sql)?;
        apply_migrations(&connection)?;
        Ok(Self { path, connection })
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn schema_version(&self) -> Result<i64, ControlPlaneError> {
        self.connection
            .query_row("SELECT MAX(version) FROM schema_migrations", [], |row| {
                row.get(0)
            })
            .map_err(ControlPlaneError::Sql)
    }

    pub fn has_table(&self, name: &str) -> Result<bool, ControlPlaneError> {
        self.connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type = 'table' AND name = ?1)",
                [name],
                |row| row.get(0),
            )
            .map_err(ControlPlaneError::Sql)
    }

    pub fn record_event(&self, event: &AgentEvent) -> Result<(), ControlPlaneError> {
        self.connection
            .execute(
                "INSERT OR IGNORE INTO agent_events(event_id, issue_number, agent_session_id, schema_version, category, summary, timestamp_unix_millis) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                rusqlite::params![
                    event.context.event_id,
                    event.context.issue_number,
                    event.context.session_id,
                    1_i64,
                    event.kind.category(),
                    event.kind.summary(),
                    i64::try_from(event.context.timestamp_unix_millis).unwrap_or(i64::MAX),
                ],
            )
            .map_err(ControlPlaneError::Sql)?;
        Ok(())
    }

    pub fn event_count(&self) -> Result<i64, ControlPlaneError> {
        self.connection
            .query_row("SELECT COUNT(*) FROM agent_events", [], |row| row.get(0))
            .map_err(ControlPlaneError::Sql)
    }

    pub fn create_pending_decision(
        &self,
        kind: &str,
        summary: &str,
        created_at_unix_seconds: i64,
        expires_at_unix_seconds: Option<i64>,
    ) -> Result<PendingDecision, ControlPlaneError> {
        self.connection.execute(
            "INSERT INTO pending_decisions(kind, summary, status, expires_at_unix_seconds, created_at_unix_seconds) VALUES (?1, ?2, 'pending', ?3, ?4)",
            rusqlite::params![kind, summary, expires_at_unix_seconds, created_at_unix_seconds],
        ).map_err(ControlPlaneError::Sql)?;
        Ok(PendingDecision {
            id: self.connection.last_insert_rowid(),
            kind: kind.to_owned(),
            summary: summary.to_owned(),
            expires_at_unix_seconds,
        })
    }

    pub fn pending_decisions(&self) -> Result<Vec<PendingDecision>, ControlPlaneError> {
        let mut statement = self.connection.prepare("SELECT id, kind, summary, expires_at_unix_seconds FROM pending_decisions WHERE status = 'pending' ORDER BY id").map_err(ControlPlaneError::Sql)?;
        statement
            .query_map([], |row| {
                Ok(PendingDecision {
                    id: row.get(0)?,
                    kind: row.get(1)?,
                    summary: row.get(2)?,
                    expires_at_unix_seconds: row.get(3)?,
                })
            })
            .map_err(ControlPlaneError::Sql)?
            .collect::<Result<Vec<_>, _>>()
            .map_err(ControlPlaneError::Sql)
    }

    /// Resolves only a still-pending decision, making retries idempotent.
    pub fn resolve_decision(
        &self,
        id: i64,
        outcome: &str,
        resolved_at_unix_seconds: i64,
    ) -> Result<bool, ControlPlaneError> {
        let updated = self.connection.execute(
            "UPDATE pending_decisions SET status = ?1, resolved_at_unix_seconds = ?2 WHERE id = ?3 AND status = 'pending'",
            rusqlite::params![outcome, resolved_at_unix_seconds, id],
        ).map_err(ControlPlaneError::Sql)?;
        Ok(updated == 1)
    }

    /// Exports bounded operational records as portable JSON without raw agent output.
    pub fn export_json(&self) -> Result<String, ControlPlaneError> {
        let mut statement = self.connection.prepare(
            "SELECT event_id, issue_number, agent_session_id, schema_version, category, summary, timestamp_unix_millis FROM agent_events ORDER BY timestamp_unix_millis, event_id",
        ).map_err(ControlPlaneError::Sql)?;
        let events = statement.query_map([], |row| {
            Ok(format!(
                "{{\"event_id\":{},\"issue_number\":{},\"agent_session_id\":{},\"schema_version\":{},\"category\":{},\"summary\":{},\"timestamp_unix_millis\":{}}}",
                json_string(&row.get::<_, String>(0)?),
                row.get::<_, Option<i64>>(1)?.map_or_else(|| "null".to_owned(), |value| value.to_string()),
                row.get::<_, Option<String>>(2)?.as_deref().map(json_string).unwrap_or_else(|| "null".to_owned()),
                row.get::<_, i64>(3)?,
                json_string(&row.get::<_, String>(4)?),
                json_string(&row.get::<_, String>(5)?),
                row.get::<_, i64>(6)?,
            ))
        }).map_err(ControlPlaneError::Sql)?;
        let events = events
            .collect::<Result<Vec<_>, _>>()
            .map_err(ControlPlaneError::Sql)?;
        Ok(format!(
            "{{\"schema_version\":1,\"events\":[{}]}}",
            events.join(",")
        ))
    }
}

fn apply_migrations(connection: &Connection) -> Result<(), ControlPlaneError> {
    connection
        .execute_batch(
            "
            CREATE TABLE IF NOT EXISTS schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_unix_seconds INTEGER NOT NULL
            );
            ",
        )
        .map_err(ControlPlaneError::Sql)?;
    let applied = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM schema_migrations WHERE version = 1)",
            [],
            |row| row.get::<_, bool>(0),
        )
        .map_err(ControlPlaneError::Sql)?;
    if applied {
        return Ok(());
    }
    connection
        .execute_batch(
            "
            CREATE TABLE repositories (id INTEGER PRIMARY KEY, repository TEXT NOT NULL UNIQUE, created_at_unix_seconds INTEGER NOT NULL);
            CREATE TABLE configuration_snapshots (id INTEGER PRIMARY KEY, repository_id INTEGER NOT NULL REFERENCES repositories(id), content TEXT NOT NULL, created_at_unix_seconds INTEGER NOT NULL);
            CREATE TABLE sessions (id INTEGER PRIMARY KEY, repository_id INTEGER NOT NULL REFERENCES repositories(id), issue_number INTEGER NOT NULL, state TEXT NOT NULL, branch TEXT NOT NULL, created_at_unix_seconds INTEGER NOT NULL, UNIQUE(repository_id, issue_number));
            CREATE TABLE workflow_transitions (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id), from_state TEXT, to_state TEXT NOT NULL, reason TEXT, created_at_unix_seconds INTEGER NOT NULL);
            CREATE TABLE blockers (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id), reason TEXT NOT NULL, owner TEXT NOT NULL, resolved_at_unix_seconds INTEGER);
            CREATE TABLE delivery_confirmations (id INTEGER PRIMARY KEY, session_id INTEGER NOT NULL REFERENCES sessions(id), action TEXT NOT NULL, decision TEXT NOT NULL, created_at_unix_seconds INTEGER NOT NULL);
            CREATE TABLE agent_events (event_id TEXT PRIMARY KEY, issue_number INTEGER, agent_session_id TEXT, schema_version INTEGER NOT NULL, category TEXT NOT NULL, summary TEXT NOT NULL, timestamp_unix_millis INTEGER NOT NULL);
            CREATE TABLE pending_decisions (id INTEGER PRIMARY KEY, session_id INTEGER REFERENCES sessions(id), kind TEXT NOT NULL, summary TEXT NOT NULL, status TEXT NOT NULL, expires_at_unix_seconds INTEGER, created_at_unix_seconds INTEGER NOT NULL, resolved_at_unix_seconds INTEGER);
            CREATE TABLE notification_deliveries (id INTEGER PRIMARY KEY, decision_id INTEGER REFERENCES pending_decisions(id), channel TEXT NOT NULL, idempotency_key TEXT NOT NULL UNIQUE, status TEXT NOT NULL, created_at_unix_seconds INTEGER NOT NULL, acknowledged_at_unix_seconds INTEGER);
            CREATE TABLE scheduled_jobs (id INTEGER PRIMARY KEY, repository_id INTEGER NOT NULL REFERENCES repositories(id), kind TEXT NOT NULL, idempotency_key TEXT NOT NULL UNIQUE, status TEXT NOT NULL, last_run_at_unix_seconds INTEGER, result_summary TEXT);
            CREATE TABLE webhook_deliveries (id INTEGER PRIMARY KEY, repository_id INTEGER NOT NULL REFERENCES repositories(id), delivery_id TEXT NOT NULL UNIQUE, event_type TEXT NOT NULL, status TEXT NOT NULL, received_at_unix_seconds INTEGER NOT NULL, reconciliation_summary TEXT);
            INSERT INTO schema_migrations(version, applied_at_unix_seconds) VALUES (1, strftime('%s', 'now'));
            ",
        )
        .map_err(ControlPlaneError::Sql)
}

#[derive(Debug)]
pub enum ControlPlaneError {
    Io(std::io::Error),
    Sql(rusqlite::Error),
}

impl std::fmt::Display for ControlPlaneError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "control-plane storage error: {error}"),
            Self::Sql(error) => write!(formatter, "control-plane database error: {error}"),
        }
    }
}

impl std::error::Error for ControlPlaneError {}

fn json_string(value: &str) -> String {
    format!(
        "\"{}\"",
        value
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t")
    )
}
