//! Append-only local evidence for consequential Git and GitHub actions.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// One operator-authorized external mutation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AuditEvent {
    pub action: String,
    pub target: String,
    pub actor: String,
    pub timestamp_unix_seconds: u64,
}

impl AuditEvent {
    pub fn new(
        action: impl Into<String>,
        target: impl Into<String>,
        actor: impl Into<String>,
        timestamp_unix_seconds: u64,
    ) -> Self {
        Self {
            action: action.into(),
            target: target.into(),
            actor: actor.into(),
            timestamp_unix_seconds,
        }
    }

    pub fn to_json(&self) -> String {
        format!(
            "{{\"action\":{},\"target\":{},\"actor\":{},\"timestamp_unix_seconds\":{}}}",
            quote(&self.action),
            quote(&self.target),
            quote(&self.actor),
            self.timestamp_unix_seconds
        )
    }
}

/// Stores JSONL audit events under `.conveyor/audit/`.
pub struct AuditLog {
    root: PathBuf,
}

impl AuditLog {
    pub fn new(repository_root: impl AsRef<Path>) -> Self {
        Self {
            root: repository_root.as_ref().join(".conveyor/audit"),
        }
    }

    pub fn append(&self, issue_number: u64, event: &AuditEvent) -> std::io::Result<()> {
        fs::create_dir_all(&self.root)?;
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(self.root.join(format!("{issue_number}.jsonl")))?;
        writeln!(file, "{}", event.to_json())
    }
}

fn quote(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for character in value.chars() {
        match character {
            '\\' => escaped.push_str("\\\\"),
            '\"' => escaped.push_str("\\\""),
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
