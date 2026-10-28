//! Parsed command dispatch.

pub(crate) mod audit;
pub(crate) mod decisions;
pub(crate) mod sessions;
pub(crate) mod setup;
pub(crate) mod tui;
pub(crate) mod workflow;

use std::path::Path;

use conveyor_core::control_plane::ControlPlaneStore;

use crate::cli::{
    AgentCommand, Cli, Commands, DecisionCommand, EventFormat, Format, IssueCommand, PrCommand,
};

pub(crate) fn execute(cli: Cli, directory: &Path) -> Result<String, String> {
    let Cli {
        command,
        event_format,
    } = cli;
    match command {
        Commands::Init(args) => setup::initialize(&args.repository, &args.target_branch, directory),
        Commands::Doctor => setup::doctor(directory),
        Commands::Status(args) => sessions::status(args.issue, directory),
        Commands::Tui => tui::run(directory),
        Commands::Block(args) => sessions::block(args.number, &args.reason, &args.owner, directory),
        Commands::Resume(args) => sessions::resume(args.number, &args.to, directory),
        Commands::Audit(args) => audit::read(args.number, directory),
        Commands::Export => ControlPlaneStore::open(directory)
            .and_then(|store| store.export_json())
            .map_err(|error| error.to_string()),
        Commands::Decision(args) => match args.command {
            DecisionCommand::List => decisions::list(directory),
            DecisionCommand::Answer { id, outcome } => decisions::answer(id, &outcome, directory),
        },
        command => {
            let mut arguments = arguments(command);
            arguments.extend([
                "--event-format".to_owned(),
                event_format_name(event_format).to_owned(),
            ]);
            workflow::run(arguments, directory)
        }
    }
}

fn event_format_name(format: EventFormat) -> &'static str {
    match format {
        EventFormat::Human => "human",
        EventFormat::Jsonl => "jsonl",
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
        Commands::Status(_) | Commands::Tui | Commands::Block(_) | Commands::Resume(_) => {
            unreachable!("session commands are dispatched directly")
        }
        Commands::Audit(_) | Commands::Export | Commands::Decision(_) => {
            unreachable!("direct commands are dispatched directly")
        }
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
