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
    pub agent_commands: Vec<AgentCommand>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConveyorConfig {
    pub version: u32,
    pub target_branch: String,
    pub branch_pattern: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GithubConfig {
    pub repository: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct WorkflowConfig {
    pub ready_label: String,
    pub in_progress_label: String,
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

/// A named, repository-approved direct CLI adapter template.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentCommand {
    pub name: String,
    pub program: String,
    pub args: Vec<String>,
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
                branch_pattern: "feat/{issue}-build".to_owned(),
            },
            github: GithubConfig {
                repository: repository.into(),
            },
            workflow: WorkflowConfig {
                ready_label: "ready-for-build".to_owned(),
                in_progress_label: "in-progress".to_owned(),
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
            agent_commands: Vec::new(),
        }
    }

    /// Finds the explicit direct-command template selected by an agent role.
    pub fn agent_command(&self, name: &str) -> Option<&AgentCommand> {
        self.agent_commands
            .iter()
            .find(|command| command.name == name)
    }

    /// Expands the repository's conventional branch name for one issue.
    pub fn branch_for_issue(&self, issue_number: u64) -> Result<String, ConfigError> {
        validate_branch_pattern(&self.conveyor.branch_pattern)?;
        Ok(self
            .conveyor
            .branch_pattern
            .replace("{issue}", &issue_number.to_string()))
    }

    /// Serializes the supported configuration surface as stable TOML.
    pub fn to_toml(&self) -> Result<String, ConfigError> {
        if self.conveyor.version != 1 {
            return Err(ConfigError::UnsupportedVersion(self.conveyor.version));
        }
        validate_branch_pattern(&self.conveyor.branch_pattern)?;
        let quoted_actions = self
            .workflow
            .require_confirmation_for
            .iter()
            .map(|action| format!("\"{}\"", action.as_str()))
            .collect::<Vec<_>>()
            .join(", ");
        let checks = self
            .checks
            .iter()
            .map(|check| {
                format!(
                    "\n[[checks]]\nname = {}\ncommand = {}\nrequired = {}\n",
                    quote(&check.name),
                    quote(&check.command),
                    check.required
                )
            })
            .collect::<String>();
        let agent_commands = self
            .agent_commands
            .iter()
            .map(|command| {
                let args = command
                    .args
                    .iter()
                    .map(|argument| quote(argument))
                    .collect::<Vec<_>>()
                    .join(", ");
                format!(
                    "\n[[agent_commands]]\nname = {}\nprogram = {}\nargs = [{}]\n",
                    quote(&command.name),
                    quote(&command.program),
                    args
                )
            })
            .collect::<String>();
        Ok(format!(
            "[conveyor]\nversion = {}\ntarget_branch = {}\nbranch_pattern = {}\n\n[github]\nrepository = {}\n\n[workflow]\nready_label = {}\nin_progress_label = {}\nblocked_label = {}\ndraft_prs_only = {}\nrequire_confirmation_for = [{}]\n\n[agents]\nplanner = {}\nimplementer = {}\nreviewer = {}\n{}{}",
            self.conveyor.version,
            quote(&self.conveyor.target_branch),
            quote(&self.conveyor.branch_pattern),
            quote(&self.github.repository),
            quote(&self.workflow.ready_label),
            quote(&self.workflow.in_progress_label),
            quote(&self.workflow.blocked_label),
            self.workflow.draft_prs_only,
            quoted_actions,
            quote(&self.agents.planner),
            quote(&self.agents.implementer),
            quote(&self.agents.reviewer),
            checks,
            agent_commands,
        ))
    }

    /// Parses the deterministic TOML produced by [`Config::to_toml`].
    pub fn from_toml(source: &str) -> Result<Self, ConfigError> {
        let mut section = "";
        let mut values = std::collections::BTreeMap::new();
        let mut check_values = Vec::<std::collections::BTreeMap<String, String>>::new();
        let mut agent_command_values = Vec::<std::collections::BTreeMap<String, String>>::new();
        for raw_line in source.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == "[[checks]]" {
                section = "checks";
                check_values.push(std::collections::BTreeMap::new());
                continue;
            }
            if line == "[[agent_commands]]" {
                section = "agent_commands";
                agent_command_values.push(std::collections::BTreeMap::new());
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
            if section == "checks" {
                check_values
                    .last_mut()
                    .expect("checks section has an entry")
                    .insert(key.trim().to_owned(), value.trim().to_owned());
            } else if section == "agent_commands" {
                agent_command_values
                    .last_mut()
                    .expect("agent_commands section has an entry")
                    .insert(key.trim().to_owned(), value.trim().to_owned());
            } else {
                values.insert(format!("{section}.{}", key.trim()), value.trim().to_owned());
            }
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
        let branch_pattern = values
            .get("conveyor.branch_pattern")
            .map(|value| parse_string(value))
            .transpose()?
            .unwrap_or_else(|| "feat/{issue}-build".to_owned());
        validate_branch_pattern(&branch_pattern)?;
        Ok(Self {
            conveyor: ConveyorConfig {
                version,
                target_branch: parse_string(required(&values, "conveyor.target_branch")?)?,
                branch_pattern,
            },
            github: GithubConfig {
                repository: parse_string(required(&values, "github.repository")?)?,
            },
            workflow: WorkflowConfig {
                ready_label: parse_string(required(&values, "workflow.ready_label")?)?,
                in_progress_label: values
                    .get("workflow.in_progress_label")
                    .map(|value| parse_string(value))
                    .transpose()?
                    .unwrap_or_else(|| "in-progress".to_owned()),
                blocked_label: parse_string(required(&values, "workflow.blocked_label")?)?,
                draft_prs_only: parse_bool(required(&values, "workflow.draft_prs_only")?)?,
                require_confirmation_for: actions,
            },
            agents: AgentConfig {
                planner: parse_string(required(&values, "agents.planner")?)?,
                implementer: parse_string(required(&values, "agents.implementer")?)?,
                reviewer: parse_string(required(&values, "agents.reviewer")?)?,
            },
            checks: check_values
                .iter()
                .map(parse_check)
                .collect::<Result<Vec<_>, _>>()?,
            agent_commands: agent_command_values
                .iter()
                .map(parse_agent_command)
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

fn validate_branch_pattern(pattern: &str) -> Result<(), ConfigError> {
    if pattern.contains("{issue}")
        && !pattern.is_empty()
        && !pattern.chars().any(char::is_whitespace)
    {
        Ok(())
    } else {
        Err(ConfigError::InvalidValue {
            key: "conveyor.branch_pattern",
            value: pattern.to_owned(),
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

fn parse_check(values: &std::collections::BTreeMap<String, String>) -> Result<Check, ConfigError> {
    let name = check_required(values, "name")?;
    let command = check_required(values, "command")?;
    let required_value = check_required(values, "required")?;
    Ok(Check {
        name: parse_string(name)?,
        command: parse_string(command)?,
        required: required_value
            .parse()
            .map_err(|_| ConfigError::InvalidValue {
                key: "checks.required",
                value: required_value.to_owned(),
            })?,
    })
}

fn parse_agent_command(
    values: &std::collections::BTreeMap<String, String>,
) -> Result<AgentCommand, ConfigError> {
    Ok(AgentCommand {
        name: parse_string(check_required(values, "name")?)?,
        program: parse_string(check_required(values, "program")?)?,
        args: parse_array(check_required(values, "args")?)?,
    })
}

fn check_required<'a>(
    values: &'a std::collections::BTreeMap<String, String>,
    key: &str,
) -> Result<&'a str, ConfigError> {
    values
        .get(key)
        .map(String::as_str)
        .ok_or_else(|| ConfigError::InvalidValue {
            key: "checks",
            value: format!("missing {key}"),
        })
}
