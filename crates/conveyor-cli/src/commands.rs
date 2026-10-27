//! Parsed command dispatch.

pub(crate) mod audit;
pub(crate) mod sessions;
pub(crate) mod setup;
pub(crate) mod workflow;

use std::path::Path;

use crate::cli::{AgentCommand, Cli, Commands, Format, IssueCommand, PrCommand};

pub(crate) fn execute(cli: Cli, directory: &Path) -> Result<String, String> {
    match cli.command {
        Commands::Init(args) => setup::initialize(&args.repository, &args.target_branch, directory),
        Commands::Doctor => setup::doctor(directory),
        Commands::Status(args) => sessions::status(args.issue, directory),
        Commands::Block(args) => sessions::block(args.number, &args.reason, &args.owner, directory),
        Commands::Resume(args) => sessions::resume(args.number, &args.to, directory),
        Commands::Audit(args) => audit::read(args.number, directory),
        command => workflow::run(arguments(command), directory),
    }
}

fn format(format: Format) -> &'static str {
    match format {
        Format::Human => "human",
        Format::Json => "json",
    }
}

fn arguments(command: Commands) -> Vec<String> {
    match command {
        Commands::Init(_) | Commands::Doctor => {
            unreachable!("setup commands are dispatched directly")
        }
        Commands::Status(_) | Commands::Block(_) | Commands::Resume(_) => {
            unreachable!("session commands are dispatched directly")
        }
        Commands::Audit(_) => unreachable!("audit commands are dispatched directly"),
        Commands::Agent(args) => match args.command {
            AgentCommand::Continue { number, prompt } => std::iter::once("agent".into())
                .chain(["continue".into(), number.to_string()])
                .chain(prompt)
                .collect(),
        },
        Commands::Issue(args) => issue_arguments(args.command),
        Commands::Handoff(args) => vec![
            "handoff".into(),
            args.number.to_string(),
            "--role".into(),
            args.role,
        ],
        Commands::Build(args) => std::iter::once("build".into())
            .chain(args.arguments)
            .collect(),
        Commands::Verify(args) => vec![
            "verify".into(),
            "--format".into(),
            format(args.format).into(),
        ]
        .into_iter()
        .chain(args.yes.then_some("--yes".into()))
        .collect(),
        Commands::Review(args) => std::iter::once("review".into())
            .chain([args.number.to_string()])
            .chain(
                args.agent
                    .into_iter()
                    .flat_map(|agent| ["--agent".into(), agent]),
            )
            .collect(),
        Commands::Pr(args) => match args.command {
            PrCommand::Open {
                number,
                waive_by,
                yes,
            } => std::iter::once("pr".into())
                .chain(["open".into(), number.to_string()])
                .chain(
                    waive_by
                        .into_iter()
                        .flat_map(|name| ["--waive-by".into(), name]),
                )
                .chain(yes.then_some("--yes".into()))
                .collect(),
        },
    }
}

fn issue_arguments(command: IssueCommand) -> Vec<String> {
    match command {
        IssueCommand::Scaffold { path } => vec!["issue".into(), "scaffold".into(), path],
        IssueCommand::Create {
            title,
            body_file,
            yes,
        } => std::iter::once("issue".into())
            .chain([
                "create".into(),
                "--title".into(),
                title,
                "--body-file".into(),
                body_file,
            ])
            .chain(yes.then_some("--yes".into()))
            .collect(),
        IssueCommand::Adapt { number, apply, yes } => std::iter::once("issue".into())
            .chain(["adapt".into(), number.to_string()])
            .chain(apply.then_some("--apply".into()))
            .chain(yes.then_some("--yes".into()))
            .collect(),
        IssueCommand::Validate {
            number,
            format: report_format,
        } => vec![
            "issue".into(),
            "validate".into(),
            number.to_string(),
            "--format".into(),
            format(report_format).into(),
        ],
    }
}
