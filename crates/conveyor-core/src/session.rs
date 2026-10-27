//! Inspectable, short-lived local records for active Conveyor work.

use std::fmt;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::workflow::{IssueState, TransitionError};

/// Local execution metadata for one issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SessionRecord {
    pub issue_number: u64,
    pub state: IssueState,
    pub branch: String,
    pub blocker: Option<String>,
    pub blocker_owner: Option<String>,
    pub agent_command: Option<String>,
    pub agent_exit_code: Option<i32>,
    pub acp_agent: Option<String>,
    pub acp_session_id: Option<String>,
    pub require_push_confirmation: bool,
    pub require_pr_confirmation: bool,
}

impl SessionRecord {
    pub fn new(issue_number: u64, state: IssueState, branch: impl Into<String>) -> Self {
        Self {
            issue_number,
            state,
            branch: branch.into(),
            blocker: None,
            blocker_owner: None,
            agent_command: None,
            agent_exit_code: None,
            acp_agent: None,
            acp_session_id: None,
            require_push_confirmation: true,
            require_pr_confirmation: true,
        }
    }

    /// Records the controlled local agent invocation and its terminal status.
    pub fn record_agent_outcome(&mut self, command: impl Into<String>, exit_code: Option<i32>) {
        self.agent_command = Some(command.into());
        self.agent_exit_code = exit_code;
    }

    /// Retains an ACP provider session identifier for a later continuation.
    pub fn record_acp_session(&mut self, agent: impl Into<String>, session_id: impl Into<String>) {
        self.acp_agent = Some(agent.into());
        self.acp_session_id = Some(session_id.into());
    }

    /// Snapshots delivery consent policy when the work session begins.
    pub fn set_delivery_confirmation_policy(&mut self, push: bool, open_pr: bool) {
        self.require_push_confirmation = push;
        self.require_pr_confirmation = open_pr;
    }

    /// Marks a session blocked with the decision owner responsible for release.
    pub fn block(
        &mut self,
        reason: impl Into<String>,
        owner: impl Into<String>,
    ) -> Result<(), TransitionError> {
        self.state = self.state.transition_to(IssueState::Blocked)?;
        self.blocker = Some(reason.into());
        self.blocker_owner = Some(owner.into());
        Ok(())
    }

    /// Clears a blocker only when the workflow permits the selected next state.
    pub fn resume(&mut self, next: IssueState) -> Result<(), TransitionError> {
        self.state = self.state.transition_to(next)?;
        self.blocker = None;
        self.blocker_owner = None;
        Ok(())
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
    let blocker_owner = record
        .blocker_owner
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
    let acp_agent = record
        .acp_agent
        .as_deref()
        .map(json_string)
        .unwrap_or_else(|| "null".to_owned());
    let acp_session_id = record
        .acp_session_id
        .as_deref()
        .map(json_string)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        "{{\n  \"issue_number\": {},\n  \"state\": {},\n  \"branch\": {},\n  \"blocker\": {},\n  \"blocker_owner\": {},\n  \"agent_command\": {},\n  \"agent_exit_code\": {},\n  \"acp_agent\": {},\n  \"acp_session_id\": {},\n  \"require_push_confirmation\": {},\n  \"require_pr_confirmation\": {}\n}}\n",
        record.issue_number,
        json_string(record.state.as_str()),
        json_string(&record.branch),
        blocker,
        blocker_owner,
        agent_command,
        agent_exit_code,
        acp_agent,
        acp_session_id,
        record.require_push_confirmation,
        record.require_pr_confirmation
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
    let blocker_owner = optional_value(source, "blocker_owner")?
        .filter(|value| *value != "null")
        .map(unquote)
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
    let acp_agent = optional_value(source, "acp_agent")?
        .filter(|value| *value != "null")
        .map(unquote)
        .transpose()?;
    let acp_session_id = optional_value(source, "acp_session_id")?
        .filter(|value| *value != "null")
        .map(unquote)
        .transpose()?;
    let require_push_confirmation = optional_value(source, "require_push_confirmation")?
        .map(parse_bool)
        .transpose()?
        .unwrap_or(true);
    let require_pr_confirmation = optional_value(source, "require_pr_confirmation")?
        .map(parse_bool)
        .transpose()?
        .unwrap_or(true);
    Ok(SessionRecord {
        issue_number,
        state,
        branch,
        blocker,
        blocker_owner,
        agent_command,
        agent_exit_code,
        acp_agent,
        acp_session_id,
        require_push_confirmation,
        require_pr_confirmation,
    })
}

fn parse_bool(value: &str) -> Result<bool, SessionError> {
    match value {
        "true" => Ok(true),
        "false" => Ok(false),
        _ => Err(SessionError::InvalidRecord(format!(
            "expected boolean: {value}"
        ))),
    }
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

fn unquote(value: &str) -> Result<String, SessionError> {
    let value = value
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
        .ok_or_else(|| SessionError::InvalidRecord(format!("expected JSON string: {value}")))?;
    let mut decoded = String::with_capacity(value.len());
    let mut characters = value.chars();
    while let Some(character) = characters.next() {
        if character != '\\' {
            decoded.push(character);
            continue;
        }
        let escaped = characters
            .next()
            .ok_or_else(|| SessionError::InvalidRecord("unterminated JSON escape".to_owned()))?;
        match escaped {
            '\\' => decoded.push('\\'),
            '\"' => decoded.push('\"'),
            'n' => decoded.push('\n'),
            'r' => decoded.push('\r'),
            't' => decoded.push('\t'),
            'u' => {
                let digits = characters.by_ref().take(4).collect::<String>();
                if digits.len() != 4 {
                    return Err(SessionError::InvalidRecord(
                        "incomplete Unicode JSON escape".to_owned(),
                    ));
                }
                let code = u32::from_str_radix(&digits, 16).map_err(|_| {
                    SessionError::InvalidRecord("invalid Unicode JSON escape".to_owned())
                })?;
                let decoded_character = char::from_u32(code).ok_or_else(|| {
                    SessionError::InvalidRecord("invalid Unicode JSON scalar".to_owned())
                })?;
                decoded.push(decoded_character);
            }
            other => {
                return Err(SessionError::InvalidRecord(format!(
                    "unknown JSON escape: \\{other}"
                )));
            }
        }
    }
    Ok(decoded)
}
