//! Local session lifecycle commands.

use std::path::Path;

use conveyor_core::session::{SessionRecord, SessionStore};
use conveyor_core::workflow::IssueState;

pub(crate) fn status(issue: Option<u64>, directory: &Path) -> Result<String, String> {
    let store = SessionStore::new(directory);
    match issue {
        None => {
            let sessions = store.list().map_err(|error| error.to_string())?;
            if sessions.is_empty() {
                Ok("status: no active local sessions".to_owned())
            } else {
                Ok(sessions
                    .iter()
                    .map(|session| render_session(session, directory))
                    .collect::<Vec<_>>()
                    .join("\n\n"))
            }
        }
        Some(issue_number) => Ok(render_session(
            &load_session(&store, issue_number)?,
            directory,
        )),
    }
}

pub(crate) fn block(
    number: u64,
    reason: &str,
    owner: &str,
    directory: &Path,
) -> Result<String, String> {
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, number)?;
    session
        .block(reason, owner)
        .map_err(|error| format!("could not block issue #{number}: {error}"))?;
    store.save(&session).map_err(|error| error.to_string())?;
    crate::commands::audit::record(
        directory,
        number,
        "record_blocker",
        format!("{reason} (owner: {owner})"),
    )?;
    Ok(format!("issue #{number} is blocked; owner: {owner}"))
}

pub(crate) fn resume(number: u64, target: &str, directory: &Path) -> Result<String, String> {
    let target = IssueState::parse(target).ok_or("resume target must be a known workflow state")?;
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, number)?;
    session
        .resume(target)
        .map_err(|error| format!("could not resume issue #{number}: {error}"))?;
    store.save(&session).map_err(|error| error.to_string())?;
    crate::commands::audit::record(directory, number, "resume_session", target.as_str())?;
    Ok(format!("issue #{number} resumed to {}", target.as_str()))
}

fn load_session(store: &SessionStore, issue_number: u64) -> Result<SessionRecord, String> {
    store
        .load(issue_number)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("status: no local session for issue #{issue_number}"))
}

fn render_session(session: &SessionRecord, directory: &Path) -> String {
    crate::commands::workflow::render_session(session, directory)
}

#[cfg(test)]
mod tests {
    use super::status;

    #[test]
    fn status_reports_an_empty_session_store() {
        let directory =
            std::env::temp_dir().join(format!("conveyor-sessions-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();
        assert_eq!(
            status(None, &directory).unwrap(),
            "status: no active local sessions"
        );
        std::fs::remove_dir_all(directory).unwrap();
    }
}
