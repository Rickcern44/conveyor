//! Durable, structured verification evidence.

use crate::verification::CheckResult;
use crate::verification::{Outcome, VerificationReport};
use std::fmt;

/// The revision and verification results attached to a local delivery attempt.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EvidenceReport {
    pub revision: String,
    pub verification: VerificationReport,
}

impl EvidenceReport {
    pub fn new(revision: impl Into<String>, verification: VerificationReport) -> Self {
        Self {
            revision: revision.into(),
            verification,
        }
    }

    /// Renders a stable JSON artifact suitable for local inspection or a PR body.
    pub fn to_json(&self) -> String {
        let checks = self
            .verification
            .results
            .iter()
            .map(|result| {
                let outcome = match result.outcome {
                    Outcome::Passed => "passed",
                    Outcome::Failed => "failed",
                };
                format!(
                    "{{\"name\":{},\"command\":{},\"required\":{},\"outcome\":{},\"exit_code\":{},\"duration_ms\":{}}}",
                    json_string(&result.name),
                    json_string(&result.command),
                    result.required,
                    json_string(outcome),
                    result.exit_code.map_or_else(|| "null".to_owned(), |code| code.to_string()),
                    result.duration_ms
                )
            })
            .collect::<Vec<_>>()
            .join(",");
        format!(
            "{{\"revision\":{},\"passed_required_checks\":{},\"checks\":[{}]}}",
            json_string(&self.revision),
            self.verification.passes_required_checks(),
            checks
        )
    }

    /// Parses JSON emitted by [`EvidenceReport::to_json`].
    pub fn from_json(source: &str) -> Result<Self, EvidenceError> {
        let revision = json_string_field(source, "revision")?;
        let checks_start = source
            .find("\"checks\":[")
            .ok_or_else(|| EvidenceError::Invalid("missing checks".to_owned()))?
            + "\"checks\":[".len();
        let checks_end = source[checks_start..]
            .rfind(']')
            .map(|index| checks_start + index)
            .ok_or_else(|| EvidenceError::Invalid("unterminated checks".to_owned()))?;
        let checks_source = source[checks_start..checks_end].trim();
        let results = if checks_source.is_empty() {
            Vec::new()
        } else {
            checks_source
                .trim_start_matches('{')
                .trim_end_matches('}')
                .split("},{")
                .map(parse_check)
                .collect::<Result<Vec<_>, _>>()?
        };
        Ok(Self::new(revision, VerificationReport { results }))
    }
}

/// Evidence could not be read from its durable JSON representation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceError {
    Invalid(String),
}

impl fmt::Display for EvidenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(detail) => write!(formatter, "invalid evidence report: {detail}"),
        }
    }
}

impl std::error::Error for EvidenceError {}

fn parse_check(source: &str) -> Result<CheckResult, EvidenceError> {
    let outcome = match json_string_field(source, "outcome")?.as_str() {
        "passed" => Outcome::Passed,
        "failed" => Outcome::Failed,
        value => return Err(EvidenceError::Invalid(format!("unknown outcome: {value}"))),
    };
    let exit_code = match json_value_field(source, "exit_code")?.as_str() {
        "null" => None,
        value => Some(
            value
                .parse()
                .map_err(|_| EvidenceError::Invalid("invalid exit_code".to_owned()))?,
        ),
    };
    Ok(CheckResult {
        name: json_string_field(source, "name")?,
        command: json_string_field(source, "command")?,
        required: json_value_field(source, "required")?
            .parse()
            .map_err(|_| EvidenceError::Invalid("invalid required".to_owned()))?,
        outcome,
        exit_code,
        duration_ms: json_value_field(source, "duration_ms")?
            .parse()
            .map_err(|_| EvidenceError::Invalid("invalid duration_ms".to_owned()))?,
    })
}

fn json_string_field(source: &str, key: &str) -> Result<String, EvidenceError> {
    let value = json_value_field(source, key)?;
    value
        .strip_prefix('\"')
        .and_then(|value| value.strip_suffix('\"'))
        .map(|value| value.replace("\\\"", "\"").replace("\\\\", "\\"))
        .ok_or_else(|| EvidenceError::Invalid(format!("{key} must be a string")))
}

fn json_value_field(source: &str, key: &str) -> Result<String, EvidenceError> {
    let needle = format!("\"{key}\":");
    let start = source
        .find(&needle)
        .map(|index| index + needle.len())
        .ok_or_else(|| EvidenceError::Invalid(format!("missing {key}")))?;
    let value = &source[start..];
    if let Some(stripped) = value.strip_prefix('\"') {
        let mut escaped = false;
        for (offset, character) in stripped.char_indices() {
            if character == '\"' && !escaped {
                return Ok(format!("\"{}\"", &stripped[..offset]));
            }
            escaped = character == '\\' && !escaped;
            if character != '\\' {
                escaped = false;
            }
        }
        return Err(EvidenceError::Invalid(format!("unterminated {key}")));
    }
    Ok(value
        .split([',', '}'])
        .next()
        .unwrap_or_default()
        .to_owned())
}

fn json_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('\"', "\\\""))
}
