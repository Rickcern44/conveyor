//! Versioned, bounded workflow events shared by agent adapters and renderers.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// The wire format emitted by [`AgentEvent::to_jsonl`].
pub const EVENT_SCHEMA_VERSION: u8 = 1;

/// Stable metadata that identifies an event without retaining agent transcripts.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EventContext {
    pub event_id: String,
    pub timestamp_unix_millis: u128,
    pub issue_number: Option<u64>,
    pub session_id: Option<String>,
}

impl EventContext {
    pub fn new(
        event_id: impl Into<String>,
        timestamp_unix_millis: u128,
        issue_number: Option<u64>,
        session_id: Option<String>,
    ) -> Self {
        Self {
            event_id: event_id.into(),
            timestamp_unix_millis,
            issue_number,
            session_id,
        }
    }
}

/// A normalized, bounded event category. Variants intentionally contain only
/// short summaries, never raw tool output or an unbounded agent transcript.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum AgentEventKind {
    SessionStarted,
    SessionResumed,
    SessionCompleted,
    SessionFailed { summary: String },
    SessionCancelled,
    AgentText { summary: String },
    PlanUpdated { steps: usize },
    ToolStarted { title: String },
    ToolProgress { summary: String },
    ToolCompleted { title: String, outcome: String },
    FileChanges { summary: String },
    PermissionRequested { summary: String },
    PermissionDecision { decision: String },
    ElicitationRequested { summary: String },
    Verification { summary: String },
    Review { summary: String },
    Delivery { summary: String },
    Blocker { summary: String },
    WorkflowState { state: String },
}

impl AgentEventKind {
    pub fn category(&self) -> &'static str {
        match self {
            Self::SessionStarted => "session.started",
            Self::SessionResumed => "session.resumed",
            Self::SessionCompleted => "session.completed",
            Self::SessionFailed { .. } => "session.failed",
            Self::SessionCancelled => "session.cancelled",
            Self::AgentText { .. } => "agent.text",
            Self::PlanUpdated { .. } => "agent.plan_updated",
            Self::ToolStarted { .. } => "tool.started",
            Self::ToolProgress { .. } => "tool.progress",
            Self::ToolCompleted { .. } => "tool.completed",
            Self::FileChanges { .. } => "file_changes.summary",
            Self::PermissionRequested { .. } => "permission.requested",
            Self::PermissionDecision { .. } => "permission.decision",
            Self::ElicitationRequested { .. } => "elicitation.requested",
            Self::Verification { .. } => "verification",
            Self::Review { .. } => "review",
            Self::Delivery { .. } => "delivery",
            Self::Blocker { .. } => "blocker",
            Self::WorkflowState { .. } => "workflow.state",
        }
    }

    pub fn summary(&self) -> String {
        match self {
            Self::SessionStarted => "session started".to_owned(),
            Self::SessionResumed => "session resumed".to_owned(),
            Self::SessionCompleted => "session completed".to_owned(),
            Self::SessionFailed { summary }
            | Self::AgentText { summary }
            | Self::ToolProgress { summary }
            | Self::FileChanges { summary }
            | Self::PermissionRequested { summary }
            | Self::ElicitationRequested { summary }
            | Self::Verification { summary }
            | Self::Review { summary }
            | Self::Delivery { summary }
            | Self::Blocker { summary } => summary.clone(),
            Self::SessionCancelled => "session cancelled".to_owned(),
            Self::PlanUpdated { steps } => format!("{steps} steps"),
            Self::ToolStarted { title } => title.clone(),
            Self::ToolCompleted { title, outcome } => format!("{title}: {outcome}"),
            Self::PermissionDecision { decision } => decision.clone(),
            Self::WorkflowState { state } => state.clone(),
        }
    }
}

/// One public event suitable for terminal rendering or JSONL streaming.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentEvent {
    pub context: EventContext,
    pub kind: AgentEventKind,
}

impl AgentEvent {
    pub fn new(context: EventContext, kind: AgentEventKind) -> Self {
        Self { context, kind }
    }

    /// Produces exactly one JSON object, without a trailing newline.
    pub fn to_jsonl(&self) -> String {
        let issue_number = self
            .context
            .issue_number
            .map_or_else(|| "null".to_owned(), |number| number.to_string());
        let session_id = self
            .context
            .session_id
            .as_deref()
            .map(json_string)
            .unwrap_or_else(|| "null".to_owned());
        format!(
            "{{\"schema_version\":{EVENT_SCHEMA_VERSION},\"event_id\":{},\"timestamp_unix_millis\":{},\"issue_number\":{issue_number},\"session_id\":{session_id},\"category\":{},\"summary\":{}}}",
            json_string(&self.context.event_id),
            self.context.timestamp_unix_millis,
            json_string(self.kind.category()),
            json_string(&self.kind.summary()),
        )
    }
}

/// Append-only, inspectable storage for normalized event summaries.
pub struct EventLog {
    repository_root: PathBuf,
    root: PathBuf,
}

impl EventLog {
    pub fn new(repository_root: impl AsRef<Path>) -> Self {
        Self {
            repository_root: repository_root.as_ref().to_path_buf(),
            root: repository_root.as_ref().join(".conveyor/events"),
        }
    }

    pub fn append(&self, event: &AgentEvent) -> Result<(), EventLogError> {
        fs::create_dir_all(&self.root).map_err(EventLogError::Io)?;
        let issue = event
            .context
            .issue_number
            .map_or_else(|| "unassigned".to_owned(), |issue| issue.to_string());
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(format!("{issue}.jsonl")))
            .map_err(EventLogError::Io)?;
        writeln!(file, "{}", event.to_jsonl()).map_err(EventLogError::Io)?;
        crate::control_plane::ControlPlaneStore::open(&self.repository_root)
            .and_then(|store| store.record_event(event))
            .map_err(EventLogError::ControlPlane)
    }
}

#[derive(Debug)]
pub enum EventLogError {
    Io(std::io::Error),
    ControlPlane(crate::control_plane::ControlPlaneError),
}

impl std::fmt::Display for EventLogError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "event log I/O error: {error}"),
            Self::ControlPlane(error) => write!(formatter, "event log database error: {error}"),
        }
    }
}

impl std::error::Error for EventLogError {}

fn json_string(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                use std::fmt::Write;
                write!(escaped, "\\u{:04x}", character as u32)
                    .expect("writing to a String cannot fail");
            }
            character => escaped.push(character),
        }
    }
    format!("\"{escaped}\"")
}
