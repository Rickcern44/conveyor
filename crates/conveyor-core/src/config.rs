//! Versioned repository configuration and its dependency-free TOML codec.

use std::fmt;

/// The complete contents of `.conveyor.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Config {
    pub conveyor: ConveyorConfig,
    pub github: GithubConfig,
    pub workflow: WorkflowConfig,
    pub agents: AgentConfig,
    pub checks: Vec<Check>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConveyorConfig {
    pub version: u32,
    pub target_branch: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GithubConfig {
    pub repository: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowConfig {
    pub ready_label: String,
    pub blocked_label: String,
    pub draft_prs_only: bool,
    pub require_confirmation_for: Vec<ConfirmationAction>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentConfig {
    pub planner: String,
    pub implementer: String,
    pub reviewer: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Check {
    pub name: String,
    pub command: String,
    pub required: bool,
}

/// A side effect that requires an operator confirmation by default.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConfirmationAction {
    Push,
    OpenPr,
    Merge,
    Deploy,
}

impl ConfirmationAction {
    fn as_str(self) -> &'static str {
        match self {
            Self::Push => "push",
            Self::OpenPr => "open_pr",
            Self::Merge => "merge",
            Self::Deploy => "deploy",
        }
    }

    fn parse(value: &str) -> Result<Self, ConfigError> {
        match value {
            "push" => Ok(Self::Push),
            "open_pr" => Ok(Self::OpenPr),
            "merge" => Ok(Self::Merge),
            "deploy" => Ok(Self::Deploy),
            _ => Err(ConfigError::InvalidValue {
                key: "require_confirmation_for",
                value: value.to_owned(),
            }),
        }
    }
}

impl Config {
    /// Builds a safe initial configuration for a GitHub repository.
    pub fn new(repository: impl Into<String>, target_branch: impl Into<String>) -> Self {
        Self {
            conveyor: ConveyorConfig {
                version: 1,
                target_branch: target_branch.into(),
            },
            github: GithubConfig {
                repository: repository.into(),
            },
            workflow: WorkflowConfig {
                ready_label: "ready-for-build".to_owned(),
                blocked_label: "blocked".to_owned(),
                draft_prs_only: true,
                require_confirmation_for: vec![
                    ConfirmationAction::Push,
                    ConfirmationAction::OpenPr,
                    ConfirmationAction::Merge,
                    ConfirmationAction::Deploy,
                ],
            },
            agents: AgentConfig {
                planner: "claude".to_owned(),
                implementer: "codex".to_owned(),
                reviewer: "claude".to_owned(),
            },
            checks: Vec::new(),
        }
    }

    /// Serializes the supported configuration surface as stable TOML.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        if self.conveyor.version != 1 {
            return Err(ConfigError::UnsupportedVersion(self.conveyor.version));
        }
        let quoted_actions = self
            .workflow
            .require_confirmation_for
            .iter()
            .map(|action| format!("\"{}\"", action.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        Ok(format!(
            "[conveyor]\nversion = {}\ntarget_branch = {}\n\n[github]\nrepository = {}\n\n[workflow]\nready_label = {}\nblocked_label = {}\ndraft_prs_only = {}\nrequire_confirmation_for = [{}]\n\n[agents]\nplanner = {}\nimplementer = {}\nreviewer = {}\n",
            self.conveyor.version,
            quote(&self.conveyor.target_branch),
            quote(&self.github.repository),
            quote(&self.workflow.ready_label),
            quote(&self.workflow.blocked_label),
            self.workflow.draft_prs_only,
            quoted_actions,
            quote(&self.agents.planner),
            quote(&self.agents.implementer),
            quote(&self.agents.reviewer),
        ))
    }

    /// Parses the deterministic TOML produced by [`Config::to_toml`].
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let mut section = "";
        let mut values = std::collections::BTreeMap::new();
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line.starts_with('[') && line.ends_with(']') {
                section = &line[1..line.len() - 1];
                continue;
            }
            let (key, value) = line
                .split_once('=')
                .ok_or_else(|| ConfigError::MalformedLine {
                    line: raw_line.to_owned(),
                })?;
            values.insert(format!("{section}.{}", key.trim()), value.trim().to_owned());
        }
        let version_value = required(&values, "conveyor.version")?;
        let version = version_value
            .parse()
            .map_err(|_| ConfigError::InvalidValue {
                key: "conveyor.version",
                value: version_value.to_owned(),
            })?;
        if version != 1 {
            return Err(ConfigError::UnsupportedVersion(version));
        }
        let actions = parse_array(required(&values, "workflow.require_confirmation_for")?)?
            .iter()
            .map(|value| ConfirmationAction::parse(value))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self {
            conveyor: ConveyorConfig {
                version,
                target_branch: parse_string(required(&values, "conveyor.target_branch")?)?,
            },
            github: GithubConfig {
                repository: parse_string(required(&values, "github.repository")?)?,
            },
            workflow: WorkflowConfig {
                ready_label: parse_string(required(&values, "workflow.ready_label")?)?,
                blocked_label: parse_string(required(&values, "workflow.blocked_label")?)?,
                draft_prs_only: parse_bool(required(&values, "workflow.draft_prs_only")?)?,
                require_confirmation_for: actions,
            },
            agents: AgentConfig {
                planner: parse_string(required(&values, "agents.planner")?)?,
                implementer: parse_string(required(&values, "agents.implementer")?)?,
                reviewer: parse_string(required(&values, "agents.reviewer")?)?,
            },
            checks: Vec::new(),
        })
    }
}

/// An error encountered while loading or writing `.conveyor.toml`.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ConfigError {
    MissingKey(&'static str),
    MalformedLine { line: String },
    InvalidValue { key: &'static str, value: String },
    UnsupportedVersion(u32),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingKey(key) => write!(formatter, "missing required configuration key: {key}"),
            Self::MalformedLine { line } => write!(formatter, "invalid configuration line: {line}"),
            Self::InvalidValue { key, value } => {
                write!(formatter, "invalid value for {key}: {value}")
            }
            Self::UnsupportedVersion(version) => {
                write!(formatter, "unsupported configuration version: {version}")
            }
        }
    }
}

impl std::error::Error for ConfigError {}

fn quote(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}

fn required<'a>(
    values: &'a std::collections::BTreeMap<String, String>,
    key: &'static str,
) -> Result<&'a str, ConfigError> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or(ConfigError::MissingKey(key))
}

fn parse_string(value: &str) -> Result<String, ConfigError> {
    value
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
        .map(|value| value.replace("\\\"", "\"").replace("\\\\", "\\"))
        .ok_or_else(|| ConfigError::InvalidValue {
            key: "string",
            value: value.to_owned(),
        })
}

fn parse_array(value: &str) -> Result<Vec<String>, ConfigError> {
    let body = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .ok_or_else(|| ConfigError::InvalidValue {
            key: "require_confirmation_for",
            value: value.to_owned(),
        })?;
    if body.trim().is_empty() {
        return Ok(Vec::new());
    }
    body.split(',')
        .map(|item| parse_string(item.trim()))
        .collect()
}

fn parse_bool(value: &str) -> Result<bool, ConfigError> {
    value.parse().map_err(|_| ConfigError::InvalidValue {
        key: "workflow.draft_prs_only",
        value: value.to_owned(),
    })
}
