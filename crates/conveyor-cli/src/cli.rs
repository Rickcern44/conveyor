//! Clap command definitions for Conveyor's public terminal interface.

use clap::{Args, Parser, Subcommand, ValueEnum};

#[derive(Debug, Parser)]
#[command(
    name = "conveyor",
    version,
    about = "A local-first issue delivery workflow"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    Init(InitArgs),
    Doctor,
    Status(StatusArgs),
    Block(BlockArgs),
    Resume(ResumeArgs),
    Agent(AgentArgs),
    Issue(IssueArgs),
    Handoff(HandoffArgs),
    Build(RawArgs),
    Verify(VerifyArgs),
    Review(ReviewArgs),
    Pr(PrArgs),
    Audit(AuditArgs),
}

#[derive(Debug, Args)]
pub struct InitArgs {
    #[arg(long)]
    pub repository: String,
    #[arg(long, default_value = "main")]
    pub target_branch: String,
}

#[derive(Debug, Args)]
pub struct StatusArgs {
    pub issue: Option<u64>,
}

#[derive(Debug, Args)]
pub struct BlockArgs {
    pub number: u64,
    #[arg(long)]
    pub reason: String,
    #[arg(long)]
    pub owner: String,
}

#[derive(Debug, Args)]
pub struct ResumeArgs {
    pub number: u64,
    #[arg(long)]
    pub to: String,
}

#[derive(Debug, Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub command: AgentCommand,
}

#[derive(Debug, Subcommand)]
pub enum AgentCommand {
    Continue {
        number: u64,
        #[arg(required = true)]
        prompt: Vec<String>,
    },
}

#[derive(Debug, Args)]
pub struct IssueArgs {
    #[command(subcommand)]
    pub command: IssueCommand,
}

#[derive(Debug, Subcommand)]
pub enum IssueCommand {
    Scaffold {
        path: String,
    },
    Create {
        #[arg(long)]
        title: String,
        #[arg(long)]
        body_file: String,
        #[arg(long)]
        yes: bool,
    },
    Adapt {
        number: u64,
        #[arg(long)]
        apply: bool,
        #[arg(long)]
        yes: bool,
    },
    Validate {
        number: u64,
        #[arg(long, value_enum, default_value_t = Format::Human)]
        format: Format,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum Format {
    Human,
    Json,
}

#[derive(Debug, Args)]
pub struct HandoffArgs {
    pub number: u64,
    #[arg(long)]
    pub role: String,
}

/// Build retains a raw trailing argument list temporarily because `build <number>`
/// and `build retry <number>` intentionally share a command name. The handler
/// continues to enforce their mutually exclusive options.
#[derive(Debug, Args)]
pub struct RawArgs {
    #[arg(allow_hyphen_values = true)]
    pub arguments: Vec<String>,
}

#[derive(Debug, Args)]
pub struct VerifyArgs {
    #[arg(long, value_enum, default_value_t = Format::Human)]
    pub format: Format,
    #[arg(long)]
    pub yes: bool,
}

#[derive(Debug, Args)]
pub struct ReviewArgs {
    pub number: u64,
    #[arg(long)]
    pub agent: Option<String>,
}

#[derive(Debug, Args)]
pub struct PrArgs {
    #[command(subcommand)]
    pub command: PrCommand,
}

#[derive(Debug, Subcommand)]
pub enum PrCommand {
    Open {
        number: u64,
        #[arg(long)]
        waive_by: Option<String>,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Debug, Args)]
pub struct AuditArgs {
    pub number: u64,
}

#[cfg(test)]
mod tests {
    use clap::Parser;

    use super::{Cli, Commands, Format, IssueCommand};

    #[test]
    fn parses_nested_issue_validation_as_typed_arguments() {
        let cli = Cli::try_parse_from(["conveyor", "issue", "validate", "70", "--format", "json"])
            .unwrap();
        assert!(matches!(
            cli.command,
            Commands::Issue(super::IssueArgs {
                command: IssueCommand::Validate {
                    number: 70,
                    format: Format::Json
                }
            })
        ));
    }
}
