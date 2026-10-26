//! Inspectable, short-lived local records for active Conveyor work.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::workflow::IssueState;

/// Local execution metadata for one issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub issue_number: u64,
    pub state: IssueState,
    pub branch: String,
    pub blocker: Option<String>,
    pub agent_command: Option<String>,
    pub agent_exit_code: Option<i32>,
}

impl SessionRecord {
    pub fn new(issue_number: u64, state: IssueState, branch: impl Into<String>) -> Self {
        Self {
            issue_number,
            state,
            branch: branch.into(),
            blocker: None,
            agent_command: None,
            agent_exit_code: None,
        }
    }

    /// Records the controlled local agent invocation and its terminal status.
    pub fn record_agent_outcome(&mut self, command: impl Into<String>, exit_code: Option<i32>) {
        self.agent_command = Some(command.into());
        self.agent_exit_code = exit_code;
    }
}

/// Reads and writes `.conveyor/sessions/<issue>.json` beneath one repository.
pub struct SessionStore {
    root: PathBuf,
}

impl SessionStore {
    pub fn new(repository_root: impl AsRef<Path>) -> Self {
        Self {
            root: repository_root.as_ref().join(".conveyor/sessions"),
        }
    }

    pub fn save(&self, record: &SessionRecord) -> Result<(), SessionError> {
        fs::create_dir_all(&self.root).map_err(SessionError::Io)?;
        let path = self.path(record.issue_number);
        let temporary = self.root.join(format!(".{}.tmp", record.issue_number));
        let mut file = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&temporary)
            .map_err(SessionError::Io)?;
        file.write_all(serialize(record).as_bytes())
            .map_err(SessionError::Io)?;
        file.sync_all().map_err(SessionError::Io)?;
        fs::rename(temporary, path).map_err(SessionError::Io)
    }

    pub fn load(&self, issue_number: u64) -> Result<Option<SessionRecord>, SessionError> {
        let path = self.path(issue_number);
        match fs::read_to_string(path) {
            Ok(source) => deserialize(&source).map(Some),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(error) => Err(SessionError::Io(error)),
        }
    }

    pub fn list(&self) -> Result<Vec<SessionRecord>, SessionError> {
        let entries = match fs::read_dir(&self.root) {
            Ok(entries) => entries,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(SessionError::Io(error)),
        };
        let mut sessions = Vec::new();
        for entry in entries {
            let entry = entry.map_err(SessionError::Io)?;
            let Some(issue_number) = entry
                .path()
                .file_stem()
                .and_then(|name| name.to_str())
                .and_then(|name| name.parse::<u64>().ok())
            else {
                continue;
            };
            if let Some(session) = self.load(issue_number)? {
                sessions.push(session);
            }
        }
        sessions.sort_by_key(|session| session.issue_number);
        Ok(sessions)
    }

    fn path(&self, issue_number: u64) -> PathBuf {
        self.root.join(format!("{issue_number}.json"))
    }
}

/// An error loading or saving a local session record.
#[derive(Debug)]
pub enum SessionError {
    Io(std::io::Error),
    InvalidRecord(String),
}

impl fmt::Display for SessionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(error) => write!(formatter, "session storage error: {error}"),
            Self::InvalidRecord(detail) => write!(formatter, "invalid session record: {detail}"),
        }
    }
}

impl std::error::Error for SessionError {}

fn serialize(record: &SessionRecord) -> String {
    let blocker = record
        .blocker
        .as_deref()
        .map(json_string)
        .unwrap_or_else(|| "null".to_owned());
    let agent_command = record
        .agent_command
        .as_deref()
        .map(json_string)
        .unwrap_or_else(|| "null".to_owned());
    let agent_exit_code = record
        .agent_exit_code
        .map_or_else(|| "null".to_owned(), |code| code.to_string());
    format!(
        "{{\n  \"issue_number\": {},\n  \"state\": {},\n  \"branch\": {},\n  \"blocker\": {},\n  \"agent_command\": {},\n  \"agent_exit_code\": {}\n}}\n",
        record.issue_number,
        json_string(record.state.as_str()),
        json_string(&record.branch),
        blocker,
        agent_command,
        agent_exit_code
    )
}

fn deserialize(source: &str) -> Result<SessionRecord, SessionError> {
    let issue_number = value(source, "issue_number")?
        .parse()
        .map_err(|_| SessionError::InvalidRecord("issue_number must be an integer".to_owned()))?;
    let state_value = unquote(value(source, "state")?)?;
    let state = IssueState::parse(&state_value)
        .ok_or_else(|| SessionError::InvalidRecord(format!("unknown state: {state_value}")))?;
    let branch = unquote(value(source, "branch")?)?;
    let blocker_value = value(source, "blocker")?;
    let blocker = (blocker_value != "null")
        .then(|| unquote(blocker_value))
        .transpose()?;
    let agent_command = optional_value(source, "agent_command")?
        .filter(|value| *value != "null")
        .map(unquote)
        .transpose()?;
    let agent_exit_code = optional_value(source, "agent_exit_code")?
        .filter(|value| *value != "null")
        .map(|value| {
            value.parse().map_err(|_| {
                SessionError::InvalidRecord("agent_exit_code must be an integer".to_owned())
            })
        })
        .transpose()?;
    Ok(SessionRecord {
        issue_number,
        state,
        branch,
        blocker,
        agent_command,
        agent_exit_code,
    })
}

fn value<'a>(source: &'a str, key: &str) -> Result<&'a str, SessionError> {
    let needle = format!("\"{key}\"");
    let after_key = source
        .find(&needle)
        .and_then(|index| {
            source[index + needle.len()..]
                .split_once(':')
                .map(|(_, rest)| rest)
        })
        .ok_or_else(|| SessionError::InvalidRecord(format!("missing {key}")))?;
    Ok(after_key
        .trim_start()
        .split([',', '\n', '}'])
        .next()
        .unwrap_or_default()
        .trim())
}

fn optional_value<'a>(source: &'a str, key: &str) -> Result<Option<&'a str>, SessionError> {
    let needle = format!("\"{key}\"");
    let Some(after_key) = source.find(&needle).and_then(|index| {
        source[index + needle.len()..]
            .split_once(':')
            .map(|(_, rest)| rest)
    }) else {
        return Ok(None);
    };
    let value = after_key
        .trim_start()
        .split([',', '\n', '}'])
        .next()
        .unwrap_or_default()
        .trim();
    if value.is_empty() {
        return Err(SessionError::InvalidRecord(format!("invalid {key}")));
    }
    Ok(Some(value))
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

fn unquote(value: &str) -> Result<String, SessionError> {
    value
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
        .map(|value| value.replace("\\\"", "\"").replace("\\\\", "\\"))
        .ok_or_else(|| SessionError::InvalidRecord(format!("expected JSON string: {value}")))
}
