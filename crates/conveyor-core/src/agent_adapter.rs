//! Provider-neutral direct CLI invocation templates.

use std::fmt;

/// A command and argument template configured by a repository.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CommandTemplate {
    pub program: String,
    pub args: Vec<String>,
}

/// Values supplied by Conveyor for one controlled agent launch.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TemplateContext {
    pub packet_path: String,
    pub repository_root: String,
    pub issue_number: u64,
    pub branch: String,
    pub role: String,
}

/// The direct process invocation after placeholder expansion.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AgentInvocation {
    pub program: String,
    pub args: Vec<String>,
}

/// A template that could cause Conveyor to start an ambiguous process.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TemplateError {
    EmptyProgram,
    UnknownPlaceholder(String),
}

impl fmt::Display for TemplateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyProgram => write!(formatter, "agent command program cannot be empty"),
            Self::UnknownPlaceholder(value) => {
                write!(formatter, "unknown command placeholder: {value}")
            }
        }
    }
}

impl std::error::Error for TemplateError {}

impl CommandTemplate {
    /// Expands supported placeholders without invoking a shell.
    pub fn expand(&self, context: &TemplateContext) -> Result<AgentInvocation, TemplateError> {
        if self.program.trim().is_empty() {
            return Err(TemplateError::EmptyProgram);
        }
        Ok(AgentInvocation {
            program: expand_value(&self.program, context)?,
            args: self
                .args
                .iter()
                .map(|argument| expand_value(argument, context))
                .collect::<Result<Vec<_>, _>>()?,
        })
    }
}

fn expand_value(value: &str, context: &TemplateContext) -> Result<String, TemplateError> {
    let expanded = value
        .replace("{packet_path}", &context.packet_path)
        .replace("{repo_root}", &context.repository_root)
        .replace("{issue_number}", &context.issue_number.to_string())
        .replace("{branch}", &context.branch)
        .replace("{role}", &context.role);
    if expanded.contains('{') || expanded.contains('}') {
        return Err(TemplateError::UnknownPlaceholder(value.to_owned()));
    }
    Ok(expanded)
}
