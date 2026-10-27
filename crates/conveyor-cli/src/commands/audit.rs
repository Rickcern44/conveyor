//! Inspectable local audit history.

use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use conveyor_core::audit::{AuditEvent, AuditLog};

pub(crate) fn read(issue_number: u64, directory: &Path) -> Result<String, String> {
    let path = directory.join(format!(".conveyor/audit/{issue_number}.jsonl"));
    std::fs::read_to_string(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("audit: no local events for issue #{issue_number}")
        } else {
            format!("could not read {}: {error}", path.display())
        }
    })
}

pub(crate) fn record(
    directory: &Path,
    issue_number: u64,
    action: &str,
    target: impl Into<String>,
) -> Result<(), String> {
    let timestamp_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("could not record audit timestamp: {error}"))?
        .as_secs();
    let actor = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "local-operator".to_owned());
    AuditLog::new(directory)
        .append(
            issue_number,
            &AuditEvent::new(action, target, actor, timestamp_unix_seconds),
        )
        .map_err(|error| format!("action completed but audit recording failed: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{read, record};

    #[test]
    fn reads_events_written_for_an_issue() {
        let directory =
            std::env::temp_dir().join(format!("conveyor-audit-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        record(&directory, 70, "test_action", "target").unwrap();
        assert!(read(70, &directory).unwrap().contains("test_action"));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
