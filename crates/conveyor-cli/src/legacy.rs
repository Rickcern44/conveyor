// Legacy workflow handlers, isolated behind the typed CLI boundary.
use std::fs::OpenOptions;
#[cfg(feature = "acp")]
use std::io::IsTerminal;
use std::io::{self, Write};
use std::path::Path;
use std::process::Command;
#[cfg(feature = "acp")]
use std::sync::{Arc, Mutex};
use std::time::{SystemTime, UNIX_EPOCH};

use conveyor_core::agent_adapter::{CommandTemplate, TemplateContext};
use conveyor_core::audit::{AuditEvent, AuditLog};
use conveyor_core::build_plan::{BuildInput, prepare};
#[cfg(feature = "acp")]
use conveyor_core::config::AcpAgentCommand;
use conveyor_core::config::{Config, ConfirmationAction};
use conveyor_core::confirmation::{ConfirmationPrompt, Decision, authorize};
use conveyor_core::doctor::{Doctor, FindingKind, Probe, ProbeResult};
use conveyor_core::evidence::EvidenceReport;
use conveyor_core::issue_contract::{from_markdown, template as issue_template, validate};
use conveyor_core::pr_gate::{PrDecision, decide, delivery_summary};
use conveyor_core::review_packet::ReviewPacket;
use conveyor_core::session::{SessionRecord, SessionStore};
use conveyor_core::task_packet::{AgentRole, TaskPacket};
use conveyor_core::verification::{
    CommandResult, CommandRunner, Outcome, VerificationReport, verify,
};
use conveyor_core::workflow::IssueState;

#[cfg(feature = "acp")]
struct AcpRunOutcome {
    session_id: String,
    response: String,
}

#[cfg(feature = "acp")]
#[derive(Debug)]
enum AgentEvent {
    Message(String),
    ToolStarted(String),
    ToolUpdated(String),
    Plan(usize),
    PermissionRequested(String),
}

#[cfg(feature = "acp")]
#[derive(Clone)]
struct AgentEventRenderer {
    interactive: bool,
}

#[cfg(feature = "acp")]
impl AgentEventRenderer {
    fn for_terminal() -> Self {
        Self {
            interactive: io::stderr().is_terminal(),
        }
    }

    fn emit(&self, event: AgentEvent) {
        if !self.interactive {
            eprintln!("conveyor: ACP event {event:?}");
            return;
        }
        match event {
            AgentEvent::Message(text) => {
                eprint!("{text}");
                let _ = io::stderr().flush();
            }
            AgentEvent::ToolStarted(title) => eprintln!("\n• {title}"),
            AgentEvent::ToolUpdated(detail) => eprintln!("  {detail}"),
            AgentEvent::Plan(steps) => eprintln!("\n• Agent updated its plan ({steps} steps)"),
            AgentEvent::PermissionRequested(detail) => {
                eprintln!("\nPermission requested\n{detail}")
            }
        }
    }
}

#[cfg(feature = "acp")]
fn approve_acp_permission(
    request: &agent_client_protocol::schema::v1::RequestPermissionRequest,
    renderer: &AgentEventRenderer,
) -> Option<agent_client_protocol::schema::v1::PermissionOptionId> {
    use agent_client_protocol::schema::v1::PermissionOptionKind;

    renderer.emit(AgentEvent::PermissionRequested(format!(
        "Operation: {}",
        request
            .tool_call
            .fields
            .title
            .as_deref()
            .unwrap_or("agent tool operation")
    )));
    let allow_once = request
        .options
        .iter()
        .find(|option| option.kind == PermissionOptionKind::AllowOnce);
    let Some(option) = allow_once else {
        eprintln!("conveyor: no one-time approval option was offered; permission denied");
        return None;
    };
    let summary = format!(
        "ACP agent requests one-time permission: {}\nApprove this specific operation?",
        option.name
    );
    if TerminalPrompt.confirm(&summary) {
        Some(option.option_id.clone())
    } else {
        None
    }
}

#[cfg(feature = "acp")]
fn run_acp_agent(
    agent_command: &AcpAgentCommand,
    prompt: &str,
    directory: &Path,
    existing_session_id: Option<&str>,
) -> Result<AcpRunOutcome, String> {
    use agent_client_protocol::schema::{
        ProtocolVersion,
        v1::{
            ContentBlock, InitializeRequest, NewSessionRequest, PromptRequest,
            RequestPermissionOutcome, RequestPermissionRequest, RequestPermissionResponse,
            ResumeSessionRequest, SelectedPermissionOutcome, SessionNotification, SessionUpdate,
            TextContent,
        },
    };
    use agent_client_protocol::{AcpAgent, Client};

    let command = std::iter::once(agent_command.program.clone())
        .chain(agent_command.args.iter().cloned())
        .collect::<Vec<_>>();
    let agent = AcpAgent::from_args(command).map_err(|error| {
        format!(
            "could not configure ACP agent `{}`: {error}",
            agent_command.name
        )
    })?;
    let action = if existing_session_id.is_some() {
        "resuming"
    } else {
        "starting"
    };
    eprintln!("conveyor: {action} ACP agent `{}`...", agent_command.name);
    let existing_session_id = existing_session_id.map(str::to_owned);
    let prompt = prompt.to_owned();
    let directory = directory.to_path_buf();
    let response = Arc::new(Mutex::new(String::new()));
    let notification_response = Arc::clone(&response);
    let renderer = AgentEventRenderer::for_terminal();
    let notification_renderer = renderer.clone();
    let permission_renderer = renderer.clone();
    let result = futures::executor::block_on(
        Client
            .builder()
            .on_receive_notification(
                async move |notification: SessionNotification, _connection| {
                    match notification.update {
                        SessionUpdate::AgentMessageChunk(chunk) => {
                            if let ContentBlock::Text(text) = chunk.content {
                                notification_renderer.emit(AgentEvent::Message(text.text.clone()));
                                notification_response
                                    .lock()
                                    .expect("ACP response lock is not poisoned")
                                    .push_str(&text.text);
                            }
                        }
                        SessionUpdate::ToolCall(tool_call) => {
                            notification_renderer.emit(AgentEvent::ToolStarted(tool_call.title));
                        }
                        SessionUpdate::ToolCallUpdate(tool_call) => {
                            let title = tool_call
                                .fields
                                .title
                                .unwrap_or_else(|| format!("Tool {}", tool_call.tool_call_id));
                            let status = tool_call
                                .fields
                                .status
                                .map(|status| format!(" ({status:?})"))
                                .unwrap_or_default();
                            notification_renderer
                                .emit(AgentEvent::ToolUpdated(format!("↳ {title}{status}")));
                        }
                        SessionUpdate::Plan(plan) => {
                            notification_renderer.emit(AgentEvent::Plan(plan.entries.len()));
                        }
                        _ => {}
                    }
                    Ok(())
                },
                agent_client_protocol::on_receive_notification!(),
            )
            .on_receive_request(
                async move |request: RequestPermissionRequest, responder, _connection| {
                    let outcome = approve_acp_permission(&request, &permission_renderer).map_or(
                        RequestPermissionOutcome::Cancelled,
                        |option_id| {
                            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new(
                                option_id,
                            ))
                        },
                    );
                    responder.respond(RequestPermissionResponse::new(outcome))
                },
                agent_client_protocol::on_receive_request!(),
            )
            .name("conveyor")
            .connect_with(agent, async move |connection| {
                connection
                    .send_request(InitializeRequest::new(ProtocolVersion::V1))
                    .block_task()
                    .await?;
                let session_id = if let Some(session_id) = &existing_session_id {
                    let session_id = session_id.clone();
                    connection
                        .send_request(ResumeSessionRequest::new(session_id.clone(), &directory))
                        .block_task()
                        .await?;
                    session_id.into()
                } else {
                    connection
                        .send_request(NewSessionRequest::new(&directory))
                        .block_task()
                        .await?
                        .session_id
                };
                connection
                    .send_request(PromptRequest::new(
                        session_id.clone(),
                        vec![ContentBlock::Text(TextContent::new(prompt))],
                    ))
                    .block_task()
                    .await?;
                Ok(AcpRunOutcome {
                    session_id: session_id.to_string(),
                    response: String::new(),
                })
            }),
    )
    .map_err(|error| format!("ACP agent `{}` failed: {error}", agent_command.name));
    if result.is_ok() {
        eprintln!();
        eprintln!("conveyor: ACP agent `{}` completed", agent_command.name);
    }
    result.map(|mut outcome| {
        outcome.response = response
            .lock()
            .expect("ACP response lock is not poisoned")
            .clone();
        outcome
    })
}

#[cfg(not(feature = "acp"))]
fn acp_feature_required(agent_name: &str) -> String {
    format!(
        "agent `{agent_name}` is configured in [[acp_agents]]; rebuild Conveyor with ACP support: `cargo install --path crates/conveyor-cli --features acp --bin conveyor`"
    )
}

pub(crate) fn run<I, S>(arguments: I, directory: &Path) -> Result<String, String>
where
    I: IntoIterator<Item = S>,
    S: AsRef<str>,
{
    let arguments = arguments
        .into_iter()
        .map(|argument| argument.as_ref().to_owned())
        .collect::<Vec<_>>();
    let Some(command) = arguments.first().map(String::as_str) else {
        return Err(usage().to_owned());
    };
    match command {
        "init" => initialize(&arguments[1..], directory),
        "doctor" => doctor(directory),
        "issue" => issue(&arguments[1..], directory),
        "status" => status(&arguments[1..], directory),
        "block" => block(&arguments[1..], directory),
        "resume" => resume(&arguments[1..], directory),
        "agent" => agent(&arguments[1..], directory),
        "verify" => verify_command(&arguments[1..], directory),
        "handoff" => handoff(&arguments[1..], directory),
        "build" => build(&arguments[1..], directory),
        "review" => review(&arguments[1..], directory),
        "pr" => pr(&arguments[1..], directory),
        "audit" => audit(&arguments[1..], directory),
        "--help" | "-h" | "help" => Ok(usage().to_owned()),
        _ => Err(format!("unknown command: {command}\n\n{}", usage())),
    }
}

fn pr(arguments: &[String], directory: &Path) -> Result<String, String> {
    let (number, explicit_yes, waiver_by) = match arguments {
        [command, number] if command == "open" => (number.as_str(), false, None),
        [command, number, flag] if command == "open" && flag == "--yes" => {
            (number.as_str(), true, None)
        }
        [command, number, waiver, operator] if command == "open" && waiver == "--waive-by" => {
            (number.as_str(), false, Some(operator.as_str()))
        }
        [command, number, waiver, operator, flag]
            if command == "open" && waiver == "--waive-by" && flag == "--yes" =>
        {
            (number.as_str(), true, Some(operator.as_str()))
        }
        _ => return Err("Usage: conveyor pr open <number> [--waive-by <name>] [--yes]".to_owned()),
    };
    let issue_number = parse_issue_number(number)?;
    let configuration = load_config(directory)?;
    let mut evidence = read_evidence(directory, &configuration)?;
    ensure_evidence_is_current(directory, &evidence)?;
    let decision = decide(
        &evidence.verification,
        waiver_by.or(evidence.waiver_by.as_deref()),
    );
    if decision == PrDecision::Blocked {
        return Err(
            "required checks failed; run `conveyor verify` or use --waive-by <name>".to_owned(),
        );
    }
    let branch = current_branch(directory)?;
    if branch == configuration.conveyor.target_branch {
        return Err("cannot open a PR from the target branch".to_owned());
    }
    let review_path = directory.join(format!(".conveyor/reviews/{issue_number}.md"));
    if !review_path.is_file() {
        return Err(format!(
            "missing {}; run `conveyor review {issue_number}` first",
            review_path.display()
        ));
    }
    let waiver_summary = match &decision {
        PrDecision::Allowed => "none".to_owned(),
        PrDecision::AllowedWithWaiver(operator) => operator.clone(),
        PrDecision::Blocked => unreachable!(),
    };
    let review_body = std::fs::read_to_string(&review_path)
        .map_err(|error| format!("could not read {}: {error}", review_path.display()))?;
    let pr_body_source = match &configuration.conveyor.pr_template_path {
        Some(path) => {
            let template_path = directory.join(path);
            let template = std::fs::read_to_string(&template_path).map_err(|error| {
                format!(
                    "could not read PR template {}: {error}",
                    template_path.display()
                )
            })?;
            format!("{template}\n\n{review_body}")
        }
        None => review_body,
    };
    let delivery_path = review_path.with_file_name(format!("{issue_number}-delivery.md"));
    std::fs::write(
        &delivery_path,
        delivery_summary(&pr_body_source, Some(&waiver_summary)),
    )
    .map_err(|error| format!("could not write {}: {error}", delivery_path.display()))?;
    let summary = format!(
        "Push branch: {branch}\nCreate draft PR: {branch} -> {}\nIssue: #{issue_number}\nVerification waiver: {waiver_summary}",
        configuration.conveyor.target_branch
    );
    let active_session = SessionStore::new(directory)
        .load(issue_number)
        .map_err(|error| error.to_string())?;
    let confirmation_required = configuration.requires_confirmation(ConfirmationAction::Push)
        || configuration.requires_confirmation(ConfirmationAction::OpenPr)
        || active_session.as_ref().is_some_and(|session| {
            session.require_push_confirmation || session.require_pr_confirmation
        });
    if confirmation_required
        && authorize(explicit_yes, &TerminalPrompt, &summary) == Decision::Declined
    {
        return Err("PR creation cancelled; no push or PR action was made".to_owned());
    }
    if let PrDecision::AllowedWithWaiver(operator) = &decision {
        evidence.record_waiver(operator);
        write_evidence(directory, &configuration, &evidence)?;
        record_audit(directory, issue_number, "waive_verification", operator)?;
    }
    mark_session_in_review(directory, issue_number)?;
    run_command(
        "git",
        ["push", "-u", &configuration.conveyor.remote, &branch],
        directory,
        "could not push branch",
    )?;
    record_audit(
        directory,
        issue_number,
        "push_branch",
        format!("{}/{branch}", configuration.conveyor.remote),
    )?;
    let title = format!("Conveyor: issue #{issue_number}");
    let output = Command::new("gh")
        .args([
            "pr",
            "create",
            "--repo",
            &configuration.github.repository,
            "--draft",
            "--base",
            &configuration.conveyor.target_branch,
            "--head",
            &branch,
            "--title",
            &title,
            "--body-file",
            delivery_path
                .to_str()
                .ok_or("delivery summary path is not valid UTF-8")?,
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not create draft PR: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let pr_target = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    record_audit(directory, issue_number, "open_draft_pr", &pr_target)?;
    Ok(format!("created draft PR: {}", pr_target))
}

fn read_evidence(directory: &Path, configuration: &Config) -> Result<EvidenceReport, String> {
    let path = evidence_path(directory, configuration)?;
    let source = std::fs::read_to_string(&path).map_err(|error| {
        format!(
            "could not read verification evidence {}: {error}",
            path.display()
        )
    })?;
    EvidenceReport::from_json(&source)
        .map_err(|error| format!("could not parse latest verification evidence: {error}"))
}

fn open_draft_pr_after_build(
    issue_number: u64,
    branch: &str,
    configuration: &Config,
    directory: &Path,
) -> Result<String, String> {
    let body = match &configuration.conveyor.pr_template_path {
        Some(path) => std::fs::read_to_string(directory.join(path))
            .map_err(|error| format!("could not read PR template {path}: {error}"))?,
        None => format!("## Conveyor delivery\n\nIssue: #{issue_number}\n"),
    };
    let delivery_path = directory.join(format!(".conveyor/reviews/{issue_number}-delivery.md"));
    let reports = delivery_path
        .parent()
        .expect("delivery path always has a parent");
    std::fs::create_dir_all(reports)
        .map_err(|error| format!("could not create {}: {error}", reports.display()))?;
    std::fs::write(&delivery_path, delivery_summary(&body, None))
        .map_err(|error| format!("could not write {}: {error}", delivery_path.display()))?;

    run_command(
        "git",
        ["push", "-u", &configuration.conveyor.remote, branch],
        directory,
        "could not push branch",
    )?;
    record_audit(
        directory,
        issue_number,
        "push_branch",
        format!("{}/{branch}", configuration.conveyor.remote),
    )?;
    let title = format!("Conveyor: issue #{issue_number}");
    let output = Command::new("gh")
        .args([
            "pr",
            "create",
            "--repo",
            &configuration.github.repository,
            "--draft",
            "--base",
            &configuration.conveyor.target_branch,
            "--head",
            branch,
            "--title",
            &title,
            "--body-file",
            delivery_path
                .to_str()
                .ok_or("delivery summary path is not valid UTF-8")?,
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not create draft PR: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let pr_target = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    mark_session_in_review(directory, issue_number)?;
    record_audit(directory, issue_number, "open_draft_pr", &pr_target)?;
    Ok(format!("created draft PR: {pr_target}"))
}

fn current_branch(directory: &Path) -> Result<String, String> {
    git_output(
        ["branch", "--show-current"],
        directory,
        "could not inspect current branch",
    )
}

fn review(arguments: &[String], directory: &Path) -> Result<String, String> {
    let (number, requested_agent) = parse_review_arguments(arguments)?;
    let issue_number = parse_issue_number(number)?;
    let configuration = load_config(directory)?;
    let contract = from_markdown(&fetch_issue_body(number, directory)?);
    let evidence = read_evidence(directory, &configuration)?;
    ensure_evidence_is_current(directory, &evidence)?;
    let checks_passed = evidence.verification.passes_required_checks();
    let diff_summary = git_output(
        [
            "diff",
            "--stat",
            &format!("{}...HEAD", configuration.conveyor.target_branch),
        ],
        directory,
        "could not summarize diff",
    )?;
    let packet = ReviewPacket {
        issue_number,
        contract,
        diff_summary,
        evidence,
    };
    let path = write_review_packet(directory, &packet)?;
    if checks_passed {
        mark_session_in_review(directory, issue_number)?;
    }
    let agent_name = requested_agent.unwrap_or(&configuration.agents.reviewer);
    if let Some(agent) = configuration.acp_agent(agent_name) {
        #[cfg(feature = "acp")]
        {
            let _outcome = run_acp_agent(&agent, &packet.to_markdown(), directory, None)?;
            record_audit(directory, issue_number, "run_acp_reviewer", &agent.program)?;
            return Ok(format!(
                "created review packet: {}; ACP reviewer completed",
                path.display()
            ));
        }
        #[cfg(not(feature = "acp"))]
        {
            let _ = agent;
            return Err(acp_feature_required(agent_name));
        }
    }
    let Some(command) = configuration.agent_command(agent_name) else {
        return Ok(format!("created review packet: {}", path.display()));
    };
    let invocation = CommandTemplate {
        program: command.program.clone(),
        args: command.args.clone(),
    }
    .expand(&TemplateContext {
        packet_path: path.display().to_string(),
        repository_root: directory.display().to_string(),
        issue_number,
        branch: current_branch(directory)?,
        role: AgentRole::Reviewer.as_str().to_owned(),
    })
    .map_err(|error| format!("invalid reviewer command template: {error}"))?;
    let output = Command::new(&invocation.program)
        .args(&invocation.args)
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not start reviewer `{}`: {error}", invocation.program))?;
    if !output.status.success() {
        return Err(format!(
            "reviewer exited unsuccessfully: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    record_audit(directory, issue_number, "run_reviewer", &invocation.program)?;
    Ok(format!(
        "created review packet: {}; reviewer completed",
        path.display()
    ))
}

fn write_review_packet(
    directory: &Path,
    packet: &ReviewPacket,
) -> Result<std::path::PathBuf, String> {
    let reviews = directory.join(".conveyor/reviews");
    std::fs::create_dir_all(&reviews)
        .map_err(|error| format!("could not create {}: {error}", reviews.display()))?;
    let path = reviews.join(format!("{}.md", packet.issue_number));
    std::fs::write(&path, packet.to_markdown())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(path)
}

fn git_output<const N: usize>(
    arguments: [&str; N],
    directory: &Path,
    context: &str,
) -> Result<String, String> {
    let output = Command::new("git")
        .args(arguments)
        .current_dir(directory)
        .output()
        .map_err(|error| format!("{context}: {error}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
    } else {
        Err(format!(
            "{context}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn parse_build_arguments(arguments: &[String]) -> Result<(&str, bool, bool, Option<&str>), String> {
    let Some(number) = arguments.first() else {
        return Err("Usage: conveyor build <number> [--agent <name>] [--dry-run|--yes]".to_owned());
    };
    let mut explicit_yes = false;
    let mut dry_run = false;
    let mut requested_agent = None;
    let mut index = 1;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--yes" if !explicit_yes => explicit_yes = true,
            "--dry-run" if !dry_run => dry_run = true,
            "--agent" if requested_agent.is_none() => {
                let Some(agent) = arguments.get(index + 1) else {
                    return Err("--agent requires a configured agent name".to_owned());
                };
                requested_agent = Some(agent.as_str());
                index += 1;
            }
            _ => {
                return Err(
                    "Usage: conveyor build <number> [--agent <name>] [--dry-run|--yes]".to_owned(),
                );
            }
        }
        index += 1;
    }
    if dry_run && explicit_yes {
        return Err("--dry-run cannot be combined with --yes".to_owned());
    }
    Ok((number, explicit_yes, dry_run, requested_agent))
}

fn parse_review_arguments(arguments: &[String]) -> Result<(&str, Option<&str>), String> {
    match arguments {
        [number] => Ok((number, None)),
        [number, option, agent] if option == "--agent" => Ok((number, Some(agent))),
        _ => Err("Usage: conveyor review <number> [--agent <name>]".to_owned()),
    }
}

fn build(arguments: &[String], directory: &Path) -> Result<String, String> {
    if arguments
        .first()
        .is_some_and(|argument| argument == "retry")
    {
        return retry_build(&arguments[1..], directory);
    }
    let (number, explicit_yes, dry_run, requested_agent) = parse_build_arguments(arguments)?;
    let issue_number = parse_issue_number(number)?;
    let configuration = load_config(directory)?;
    let labels = issue_labels(number, &configuration, directory)?;
    let state = if labels
        .iter()
        .any(|label| label == &configuration.workflow.ready_label)
    {
        IssueState::ReadyForBuild
    } else {
        IssueState::NeedsSpec
    };
    ensure_on_target_branch(directory, &configuration.conveyor.target_branch)?;
    let plan = prepare(BuildInput {
        issue_number,
        state,
        worktree_clean: worktree_is_clean(directory)?,
        worktree_up_to_date: worktree_is_up_to_date(directory)?,
        branch: configuration
            .branch_for_issue(issue_number)
            .map_err(|error| error.to_string())?,
    })
    .map_err(|error| error.to_string())?;
    let contract = from_markdown(&fetch_issue_body(number, directory)?);
    let validation = validate(&contract);
    if !validation.is_valid() {
        return Err(format!(
            "issue #{issue_number} is not ready for build; missing:\n{}",
            validation
                .missing
                .iter()
                .map(|requirement| format!("- {}", requirement.label()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let agent_name = requested_agent.unwrap_or(&configuration.agents.implementer);
    let acp_agent = configuration.acp_agent(agent_name);
    #[cfg(not(feature = "acp"))]
    if acp_agent.is_some() {
        return Err(acp_feature_required(agent_name));
    }
    let command = configuration.agent_command(agent_name).cloned();
    let agent_program = acp_agent
        .as_ref()
        .map(|agent| agent.program.as_str())
        .or_else(|| command.as_ref().map(|agent| agent.program.as_str()))
        .ok_or_else(|| format!(
            "no [[acp_agents]] or [[agent_commands]] entry named `{agent_name}` is configured for the implementer"
        ))?
        .to_owned();
    let uses_acp = acp_agent.is_some();
    if dry_run {
        return Ok(format!(
            "build dry run for issue #{issue_number}\nbranch: {}\nGitHub label transition: {} -> {}\nagent command: {}\nno Git, GitHub, session, packet, or agent action was performed",
            plan.branch,
            configuration.workflow.ready_label,
            configuration.workflow.in_progress_label,
            agent_program
        ));
    }
    let summary = format!(
        "Issue #{issue_number}\nBranch to create: {}\nGitHub label transition: {} -> {}\nLocal session: .conveyor/sessions/{issue_number}.json\nAgent command: {}\nAfter a clean agent result: push the branch and create a draft PR",
        plan.branch,
        configuration.workflow.ready_label,
        configuration.workflow.in_progress_label,
        agent_program
    );
    if authorize(explicit_yes, &TerminalPrompt, &summary) == Decision::Declined {
        return Err("build cancelled; no Git or GitHub changes were made".to_owned());
    }
    run_command(
        "git",
        ["switch", "-c", plan.branch.as_str()],
        directory,
        "could not create build branch",
    )?;
    record_audit(directory, issue_number, "create_branch", &plan.branch)?;
    let store = SessionStore::new(directory);
    if let Err(error) = transition_issue_to_in_progress(number, &configuration, directory) {
        let mut recovery = SessionRecord::new(issue_number, IssueState::InProgress, &plan.branch);
        recovery.set_delivery_confirmation_policy(
            configuration.requires_confirmation(ConfirmationAction::Push),
            configuration.requires_confirmation(ConfirmationAction::OpenPr),
        );
        let reason = format!("GitHub label transition failed after branch creation: {error}");
        recovery
            .block(&reason, "local-operator")
            .expect("in-progress sessions may be blocked");
        let recovery_note = match store.save(&recovery) {
            Ok(()) => {
                let _ = record_audit(directory, issue_number, "record_blocker", &reason);
                format!(" A blocked recovery session was saved for issue #{issue_number}.")
            }
            Err(save_error) => format!(" Recovery session could not be saved: {save_error}."),
        };
        return Err(format!(
            "branch {} was created, but the issue transition failed: {error}.{recovery_note}",
            plan.branch,
        ));
    }
    record_audit(
        directory,
        issue_number,
        "transition_issue",
        format!(
            "#{issue_number}: {} -> {}",
            configuration.workflow.ready_label, configuration.workflow.in_progress_label
        ),
    )?;
    let mut session = SessionRecord::new(issue_number, plan.next_state, &plan.branch);
    session.set_delivery_confirmation_policy(
        configuration.requires_confirmation(ConfirmationAction::Push),
        configuration.requires_confirmation(ConfirmationAction::OpenPr),
    );
    store.save(&session).map_err(|error| {
        format!("branch and issue updated, but session could not be saved: {error}")
    })?;
    let guidance = std::fs::read_to_string(directory.join("AGENTS.md"))
        .unwrap_or_else(|_| "No repository agent guidance was found.".to_owned());
    let packet = TaskPacket::new(
        issue_number,
        AgentRole::Implementer,
        &plan.branch,
        configuration
            .commit_message_for_issue(issue_number)
            .map_err(|error| error.to_string())?,
        contract,
        guidance,
        configuration.checks.clone(),
    );
    let packet_path = write_packet(directory, &packet)?;
    let (agent_invocation, agent_exit_code, agent_failure) = if let Some(agent) = &acp_agent {
        #[cfg(feature = "acp")]
        {
            let invocation = format!("ACP {}", agent.program);
            match run_acp_agent(agent, &packet.to_markdown(), directory, None) {
                Ok(outcome) => {
                    session.record_acp_session(&agent.name, outcome.session_id);
                    (invocation, Some(0), None)
                }
                Err(error) => (invocation, None, Some(error)),
            }
        }
        #[cfg(not(feature = "acp"))]
        {
            let _ = agent;
            unreachable!("ACP agents are rejected before execution")
        }
    } else {
        let command =
            command.expect("a direct command is required when no ACP agent is configured");
        let invocation = CommandTemplate {
            program: command.program,
            args: command.args,
        }
        .expand(&TemplateContext {
            packet_path: packet_path.display().to_string(),
            repository_root: directory.display().to_string(),
            issue_number,
            branch: plan.branch.clone(),
            role: AgentRole::Implementer.as_str().to_owned(),
        })
        .map_err(|error| format!("invalid implementer command template: {error}"))?;
        let output = Command::new(&invocation.program)
            .args(&invocation.args)
            .current_dir(directory)
            .output()
            .map_err(|error| {
                format!(
                    "could not start implementer `{}`: {error}",
                    invocation.program
                )
            })?;
        let failure = (!output.status.success()).then(|| {
            format!(
                "implementer exited unsuccessfully: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
        });
        (
            std::iter::once(invocation.program.as_str())
                .chain(invocation.args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            output.status.code(),
            failure,
        )
    };
    session.record_agent_outcome(agent_invocation, agent_exit_code);
    store
        .save(&session)
        .map_err(|error| format!("agent ran, but its outcome could not be saved: {error}"))?;
    record_audit(
        directory,
        issue_number,
        if uses_acp {
            "run_acp_implementer"
        } else {
            "run_implementer"
        },
        &agent_program,
    )?;
    if let Some(error) = agent_failure {
        return Err(error);
    }
    if !worktree_is_clean(directory)? {
        return Err(
            "implementer completed, but the worktree is not clean; commit or stash the changes, then run `conveyor pr open` when ready"
                .to_owned(),
        );
    }
    let pr_result =
        open_draft_pr_after_build(issue_number, &plan.branch, &configuration, directory)?;
    Ok(format!(
        "build session started for issue #{issue_number} on {}; implementer completed\n{pr_result}",
        plan.branch
    ))
}

fn retry_build(arguments: &[String], directory: &Path) -> Result<String, String> {
    let (number, explicit_yes) = match arguments {
        [number] => (number.as_str(), false),
        [number, yes] if yes == "--yes" => (number.as_str(), true),
        _ => return Err("Usage: conveyor build retry <number> [--yes]".to_owned()),
    };
    let issue_number = parse_issue_number(number)?;
    let configuration = load_config(directory)?;
    let labels = issue_labels(number, &configuration, directory)?;
    if !labels
        .iter()
        .any(|label| label == &configuration.workflow.in_progress_label)
    {
        return Err(format!(
            "issue #{issue_number} must have `{}` before retrying the implementer",
            configuration.workflow.in_progress_label
        ));
    }
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, issue_number)?;
    if session.state != IssueState::InProgress {
        return Err(format!(
            "issue #{issue_number} is {}; resume it to in-progress before retrying",
            session.state.as_str()
        ));
    }
    if session.agent_command.is_some() {
        return Err(format!(
            "issue #{issue_number} already has an implementer outcome; use `conveyor agent continue {issue_number} <prompt>` or inspect `conveyor status {issue_number}`"
        ));
    }
    if current_branch(directory)? != session.branch {
        return Err(format!(
            "retry must run on `{}`; switch to that branch first",
            session.branch
        ));
    }
    let contract = from_markdown(&fetch_issue_body(number, directory)?);
    let validation = validate(&contract);
    if !validation.is_valid() {
        return Err(format!(
            "issue #{issue_number} is not ready for implementation"
        ));
    }
    let agent_name = &configuration.agents.implementer;
    let acp_agent = configuration.acp_agent(agent_name);
    #[cfg(not(feature = "acp"))]
    if acp_agent.is_some() {
        return Err(acp_feature_required(agent_name));
    }
    let command = configuration.agent_command(agent_name).cloned();
    let agent_program = acp_agent
        .as_ref()
        .map(|agent| agent.program.as_str())
        .or_else(|| command.as_ref().map(|agent| agent.program.as_str()))
        .ok_or_else(|| format!("no configured implementer named `{agent_name}`"))?
        .to_owned();
    let summary = format!(
        "Retry implementer for issue #{issue_number}\nBranch: {}\nAgent: {agent_program}",
        session.branch
    );
    if authorize(explicit_yes, &TerminalPrompt, &summary) == Decision::Declined {
        return Err("build retry cancelled; no agent was started".to_owned());
    }
    let guidance = std::fs::read_to_string(directory.join("AGENTS.md"))
        .unwrap_or_else(|_| "No repository agent guidance was found.".to_owned());
    let packet = TaskPacket::new(
        issue_number,
        AgentRole::Implementer,
        &session.branch,
        configuration
            .commit_message_for_issue(issue_number)
            .map_err(|error| error.to_string())?,
        contract,
        guidance,
        configuration.checks.clone(),
    );
    let packet_path = write_packet(directory, &packet)?;
    let (invocation, exit_code, failure) = if let Some(agent) = &acp_agent {
        #[cfg(feature = "acp")]
        {
            let invocation = format!("ACP {}", agent.program);
            match run_acp_agent(agent, &packet.to_markdown(), directory, None) {
                Ok(outcome) => {
                    session.record_acp_session(&agent.name, outcome.session_id);
                    (invocation, Some(0), None)
                }
                Err(error) => (invocation, None, Some(error)),
            }
        }
        #[cfg(not(feature = "acp"))]
        {
            let _ = agent;
            unreachable!("ACP agents are rejected before execution")
        }
    } else {
        let command = command.expect("direct implementer command is configured");
        let invocation = CommandTemplate {
            program: command.program,
            args: command.args,
        }
        .expand(&TemplateContext {
            packet_path: packet_path.display().to_string(),
            repository_root: directory.display().to_string(),
            issue_number,
            branch: session.branch.clone(),
            role: AgentRole::Implementer.as_str().to_owned(),
        })
        .map_err(|error| format!("invalid implementer command template: {error}"))?;
        let output = Command::new(&invocation.program)
            .args(&invocation.args)
            .current_dir(directory)
            .output()
            .map_err(|error| {
                format!(
                    "could not start implementer `{}`: {error}",
                    invocation.program
                )
            })?;
        let failure = (!output.status.success()).then(|| {
            format!(
                "implementer exited unsuccessfully: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )
        });
        (
            std::iter::once(invocation.program.as_str())
                .chain(invocation.args.iter().map(String::as_str))
                .collect::<Vec<_>>()
                .join(" "),
            output.status.code(),
            failure,
        )
    };
    session.record_agent_outcome(invocation, exit_code);
    store.save(&session).map_err(|error| error.to_string())?;
    record_audit(directory, issue_number, "retry_implementer", &agent_program)?;
    if let Some(error) = failure {
        return Err(error);
    }
    Ok(format!(
        "implementer retry completed for issue #{issue_number}"
    ))
}

struct TerminalPrompt;

impl ConfirmationPrompt for TerminalPrompt {
    fn confirm(&self, summary: &str) -> bool {
        println!("Conveyor will perform these changes:\n\n{summary}\n");
        print!("Proceed? [y/N]: ");
        if io::stdout().flush().is_err() {
            return false;
        }
        let mut response = String::new();
        io::stdin()
            .read_line(&mut response)
            .is_ok_and(|_| matches!(response.trim(), "y" | "Y" | "yes" | "YES"))
    }
}

fn issue_labels(
    number: &str,
    configuration: &Config,
    directory: &Path,
) -> Result<Vec<String>, String> {
    let output = Command::new("gh")
        .args([
            "issue",
            "view",
            number,
            "--repo",
            &configuration.github.repository,
            "--json",
            "labels",
            "--jq",
            ".labels[].name",
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not read issue #{number} labels: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::to_owned)
        .collect())
}

fn ensure_on_target_branch(directory: &Path, target_branch: &str) -> Result<(), String> {
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not inspect current branch: {error}"))?;
    let current_branch = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    (output.status.success() && current_branch == target_branch)
        .then_some(())
        .ok_or_else(|| {
            format!("build must start on target branch `{target_branch}`, found `{current_branch}`")
        })
}

fn worktree_is_clean(directory: &Path) -> Result<bool, String> {
    let output = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not inspect worktree: {error}"))?;
    if !output.status.success() {
        return Err("could not inspect worktree status".to_owned());
    }
    let has_non_conveyor_change = String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| line.get(3..))
        .any(|path| !path.starts_with(".conveyor/"));
    Ok(!has_non_conveyor_change)
}

fn worktree_is_up_to_date(directory: &Path) -> Result<bool, String> {
    let upstream = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "@{upstream}"])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not inspect upstream branch: {error}"))?;
    if !upstream.status.success() {
        return Err("target branch has no configured upstream".to_owned());
    }
    let output = Command::new("git")
        .args(["rev-list", "--count", "HEAD..@{upstream}"])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not inspect upstream revision: {error}"))?;
    let commits_behind = String::from_utf8_lossy(&output.stdout)
        .trim()
        .parse::<u64>()
        .map_err(|_| "could not determine upstream revision".to_owned())?;
    Ok(output.status.success() && commits_behind == 0)
}

fn transition_issue_to_in_progress(
    number: &str,
    configuration: &Config,
    directory: &Path,
) -> Result<(), String> {
    run_command(
        "gh",
        [
            "issue",
            "edit",
            number,
            "--repo",
            &configuration.github.repository,
            "--remove-label",
            &configuration.workflow.ready_label,
            "--add-label",
            &configuration.workflow.in_progress_label,
        ],
        directory,
        "could not transition issue to in-progress",
    )
}

fn run_command<const N: usize>(
    program: &str,
    arguments: [&str; N],
    directory: &Path,
    context: &str,
) -> Result<(), String> {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .output()
        .map_err(|error| format!("{context}: {error}"))?;
    if output.status.success() {
        Ok(())
    } else {
        Err(format!(
            "{context}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ))
    }
}

fn handoff(arguments: &[String], directory: &Path) -> Result<String, String> {
    let (number, role) = match arguments {
        [number, option, role] if option == "--role" => (number.as_str(), role.as_str()),
        _ => {
            return Err(
                "Usage: conveyor handoff <number> --role <planner|implementer|reviewer>".to_owned(),
            );
        }
    };
    let issue_number = parse_issue_number(number)?;
    let role = AgentRole::parse(role).ok_or("role must be planner, implementer, or reviewer")?;
    let contract = from_markdown(&fetch_issue_body(number, directory)?);
    let validation = validate(&contract);
    if !validation.is_valid() {
        return Err(format!(
            "issue #{issue_number} is not ready for handoff; missing:\n{}",
            validation
                .missing
                .iter()
                .map(|requirement| format!("- {}", requirement.label()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let configuration = load_config(directory)?;
    let guidance = std::fs::read_to_string(directory.join("AGENTS.md"))
        .unwrap_or_else(|_| "No repository agent guidance was found.".to_owned());
    let packet = TaskPacket::new(
        issue_number,
        role,
        configuration
            .branch_for_issue(issue_number)
            .map_err(|error| error.to_string())?,
        configuration
            .commit_message_for_issue(issue_number)
            .map_err(|error| error.to_string())?,
        contract,
        guidance,
        configuration.checks,
    );
    let path = write_packet(directory, &packet)?;
    Ok(format!("created task packet: {}", path.display()))
}

fn write_packet(directory: &Path, packet: &TaskPacket) -> Result<std::path::PathBuf, String> {
    let packets = directory.join(".conveyor/packets");
    std::fs::create_dir_all(&packets)
        .map_err(|error| format!("could not create {}: {error}", packets.display()))?;
    let filename = format!("{}-{}.md", packet.issue_number, packet.role.as_str());
    let path = packets.join(&filename);
    let temporary = packets.join(format!(".{filename}.tmp"));
    std::fs::write(&temporary, packet.to_markdown())
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("could not save {}: {error}", path.display()))?;
    Ok(path)
}

fn verify_command(arguments: &[String], directory: &Path) -> Result<String, String> {
    let (format, explicit_yes) = parse_verify_arguments(arguments)?;
    let configuration = load_config(directory)?;
    let evidence = EvidenceReport::new(
        current_revision(directory),
        verify(&LocalCommandRunner { directory }, &configuration.checks),
    );
    write_evidence(directory, &configuration, &evidence)?;
    let comment_result =
        comment_verification_on_pr(&evidence, &configuration, directory, explicit_yes)?;
    match format {
        ReportFormat::Human => Ok(format!(
            "{}\n{comment_result}",
            render_verification_human(&evidence.verification)
        )),
        ReportFormat::Json => Ok(evidence.to_json()),
    }
}

fn comment_verification_on_pr(
    evidence: &EvidenceReport,
    configuration: &Config,
    directory: &Path,
    explicit_yes: bool,
) -> Result<String, String> {
    let branch = match current_branch(directory) {
        Ok(branch) if !branch.is_empty() && branch != configuration.conveyor.target_branch => {
            branch
        }
        _ => return Ok("verification PR comment: skipped (no delivery branch)".to_owned()),
    };
    let lookup = Command::new("gh")
        .args([
            "pr",
            "view",
            &branch,
            "--repo",
            &configuration.github.repository,
            "--json",
            "url",
            "--jq",
            ".url",
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !lookup.status.success() {
        return Ok("verification PR comment: skipped (no PR for this branch)".to_owned());
    }
    let pr_url = String::from_utf8_lossy(&lookup.stdout).trim().to_owned();
    if configuration.requires_confirmation(ConfirmationAction::CommentPr)
        && authorize(
            explicit_yes,
            &TerminalPrompt,
            &format!("Post verification report to draft PR: {pr_url}"),
        ) == Decision::Declined
    {
        return Ok("verification PR comment: skipped (confirmation declined)".to_owned());
    }
    let comment_path = directory.join(".conveyor/reports/verification-comment.md");
    let reports = comment_path
        .parent()
        .expect("verification comment path always has a parent");
    std::fs::create_dir_all(reports)
        .map_err(|error| format!("could not create {}: {error}", reports.display()))?;
    std::fs::write(
        &comment_path,
        format!(
            "## Conveyor verification\n\nRevision: `{}`\n\n{}\n",
            evidence.revision,
            render_verification_human(&evidence.verification)
        ),
    )
    .map_err(|error| format!("could not write {}: {error}", comment_path.display()))?;
    let output = Command::new("gh")
        .args([
            "pr",
            "comment",
            &branch,
            "--repo",
            &configuration.github.repository,
            "--body-file",
            comment_path
                .to_str()
                .ok_or("verification comment path is not valid UTF-8")?,
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not post verification PR comment: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(format!("verification PR comment: posted to {pr_url}"))
}

fn current_revision(directory: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(directory)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map(|output| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|revision| !revision.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
}

fn ensure_evidence_is_current(directory: &Path, evidence: &EvidenceReport) -> Result<(), String> {
    let revision = current_revision(directory);
    if evidence.is_for_revision(&revision) {
        Ok(())
    } else {
        Err(format!(
            "verification evidence is stale (recorded {}, current {revision}); run `conveyor verify` again",
            evidence.revision
        ))
    }
}

fn evidence_path(directory: &Path, configuration: &Config) -> Result<std::path::PathBuf, String> {
    let relative = Path::new(&configuration.conveyor.verification_report_path);
    if relative.as_os_str().is_empty()
        || relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err("verification_report_path must be relative to the repository".to_owned());
    }
    Ok(directory.join(relative))
}

fn write_evidence(
    directory: &Path,
    configuration: &Config,
    evidence: &EvidenceReport,
) -> Result<(), String> {
    let path = evidence_path(directory, configuration)?;
    let reports = path
        .parent()
        .ok_or("verification_report_path must name a file")?;
    std::fs::create_dir_all(reports)
        .map_err(|error| format!("could not create {}: {error}", reports.display()))?;
    let temporary = path.with_extension("tmp");
    std::fs::write(&temporary, evidence.to_json())
        .map_err(|error| format!("could not write {}: {error}", temporary.display()))?;
    std::fs::rename(&temporary, &path)
        .map_err(|error| format!("could not save {}: {error}", path.display()))
}

fn load_config(directory: &Path) -> Result<Config, String> {
    let path = directory.join(".conveyor.toml");
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    Config::from_toml(&source)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))
}

struct LocalCommandRunner<'a> {
    directory: &'a Path,
}

impl CommandRunner for LocalCommandRunner<'_> {
    fn run(&self, command: &str) -> CommandResult {
        let started = std::time::Instant::now();
        let status = Command::new("/bin/sh")
            .args(["-c", command])
            .current_dir(self.directory)
            .status();
        CommandResult {
            exit_code: status.ok().and_then(|status| status.code()),
            duration_ms: started.elapsed().as_millis(),
        }
    }
}

fn render_verification_human(report: &VerificationReport) -> String {
    let status = if report.passes_required_checks() {
        "passed"
    } else {
        "failed"
    };
    let checks = report
        .results
        .iter()
        .map(|result| {
            let outcome = match result.outcome {
                Outcome::Passed => "passed",
                Outcome::Failed => "failed",
            };
            format!(
                "{}: {} (exit: {}, duration: {}ms, required: {})",
                result.name,
                outcome,
                result
                    .exit_code
                    .map_or_else(|| "unavailable".to_owned(), |code| code.to_string()),
                result.duration_ms,
                result.required
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!("verification: {status}\n{checks}")
}

fn audit(arguments: &[String], directory: &Path) -> Result<String, String> {
    let [number] = arguments else {
        return Err("Usage: conveyor audit <number>".to_owned());
    };
    let issue_number = parse_issue_number(number)?;
    let path = directory.join(format!(".conveyor/audit/{issue_number}.jsonl"));
    std::fs::read_to_string(&path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!("audit: no local events for issue #{issue_number}")
        } else {
            format!("could not read {}: {error}", path.display())
        }
    })
}

fn record_audit(
    directory: &Path,
    issue_number: u64,
    action: &str,
    target: impl Into<String>,
) -> Result<(), String> {
    let timestamp_unix_seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("could not record audit timestamp: {error}"))?
        .as_secs();
    let actor = std::env::var("USER")
        .or_else(|_| std::env::var("USERNAME"))
        .unwrap_or_else(|_| "local-operator".to_owned());
    AuditLog::new(directory)
        .append(
            issue_number,
            &AuditEvent::new(action, target, actor, timestamp_unix_seconds),
        )
        .map_err(|error| format!("action completed but audit recording failed: {error}"))
}

fn status(arguments: &[String], directory: &Path) -> Result<String, String> {
    let store = SessionStore::new(directory);
    match arguments {
        [] => {
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
        [issue] => {
            let issue_number = parse_issue_number(issue)?;
            let session = store
                .load(issue_number)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("status: no local session for issue #{issue_number}"))?;
            Ok(render_session(&session, directory))
        }
        _ => Err("Usage: conveyor status [issue]".to_owned()),
    }
}

fn agent(arguments: &[String], directory: &Path) -> Result<String, String> {
    let [command, number, prompt @ ..] = arguments else {
        return Err("Usage: conveyor agent continue <number> <prompt>".to_owned());
    };
    if command != "continue" || prompt.is_empty() {
        return Err("Usage: conveyor agent continue <number> <prompt>".to_owned());
    }
    let issue_number = parse_issue_number(number)?;
    let store = SessionStore::new(directory);
    let session = load_session(&store, issue_number)?;
    let agent_name = session
        .acp_agent
        .clone()
        .ok_or_else(|| format!("issue #{issue_number} has no resumable ACP agent session"))?;
    let session_id = session
        .acp_session_id
        .clone()
        .ok_or_else(|| format!("issue #{issue_number} has no resumable ACP session identifier"))?;
    let configuration = load_config(directory)?;
    let agent = configuration
        .acp_agent(&agent_name)
        .ok_or_else(|| format!("ACP agent `{agent_name}` is no longer configured"))?;
    #[cfg(feature = "acp")]
    {
        let mut session = session;
        let outcome = run_acp_agent(&agent, &prompt.join(" "), directory, Some(&session_id))?;
        session.record_acp_session(agent_name, outcome.session_id);
        session.record_agent_outcome(format!("ACP {} (resumed)", agent.program), Some(0));
        store.save(&session).map_err(|error| error.to_string())?;
        record_audit(directory, issue_number, "resume_acp_agent", &agent.program)?;
        Ok(format!(
            "ACP agent session continued for issue #{issue_number}"
        ))
    }
    #[cfg(not(feature = "acp"))]
    {
        let _ = (agent, session_id, session);
        Err(acp_feature_required(&agent_name))
    }
}

fn block(arguments: &[String], directory: &Path) -> Result<String, String> {
    let [number, reason_flag, reason, owner_flag, owner] = arguments else {
        return Err("Usage: conveyor block <number> --reason <text> --owner <name>".to_owned());
    };
    if reason_flag != "--reason"
        || owner_flag != "--owner"
        || reason.trim().is_empty()
        || owner.trim().is_empty()
    {
        return Err("Usage: conveyor block <number> --reason <text> --owner <name>".to_owned());
    }
    let issue_number = parse_issue_number(number)?;
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, issue_number)?;
    session
        .block(reason, owner)
        .map_err(|error| format!("could not block issue #{issue_number}: {error}"))?;
    store.save(&session).map_err(|error| error.to_string())?;
    record_audit(
        directory,
        issue_number,
        "record_blocker",
        format!("{reason} (owner: {owner})"),
    )?;
    Ok(format!("issue #{issue_number} is blocked; owner: {owner}"))
}

fn resume(arguments: &[String], directory: &Path) -> Result<String, String> {
    let [number, to_flag, target] = arguments else {
        return Err("Usage: conveyor resume <number> --to <state>".to_owned());
    };
    if to_flag != "--to" {
        return Err("Usage: conveyor resume <number> --to <state>".to_owned());
    }
    let issue_number = parse_issue_number(number)?;
    let target = IssueState::parse(target).ok_or("resume target must be a known workflow state")?;
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, issue_number)?;
    session
        .resume(target)
        .map_err(|error| format!("could not resume issue #{issue_number}: {error}"))?;
    store.save(&session).map_err(|error| error.to_string())?;
    record_audit(directory, issue_number, "resume_session", target.as_str())?;
    Ok(format!(
        "issue #{issue_number} resumed to {}",
        target.as_str()
    ))
}

fn load_session(store: &SessionStore, issue_number: u64) -> Result<SessionRecord, String> {
    store
        .load(issue_number)
        .map_err(|error| error.to_string())?
        .ok_or_else(|| format!("status: no local session for issue #{issue_number}"))
}

fn mark_session_in_review(directory: &Path, issue_number: u64) -> Result<(), String> {
    let store = SessionStore::new(directory);
    let mut session = load_session(&store, issue_number)?;
    match session.state {
        IssueState::InReview => Ok(()),
        IssueState::InProgress => {
            session.state = session
                .state
                .transition_to(IssueState::InReview)
                .map_err(|error| error.to_string())?;
            store.save(&session).map_err(|error| error.to_string())?;
            record_audit(directory, issue_number, "transition_session", "in-review")
        }
        state => Err(format!(
            "could not prepare issue #{issue_number} for review from state {}",
            state.as_str()
        )),
    }
}

fn render_session(session: &SessionRecord, directory: &Path) -> String {
    let blocker = session.blocker.as_deref().unwrap_or("none");
    let blocker_owner = session.blocker_owner.as_deref().unwrap_or("none");
    let current_branch = current_branch(directory).unwrap_or_else(|_| "unknown".to_owned());
    let required_checks =
        match load_config(directory).and_then(|config| read_evidence(directory, &config)) {
            Ok(evidence) if !evidence.is_for_revision(&current_revision(directory)) => {
                "stale; run verify".to_owned()
            }
            Ok(evidence) if evidence.verification.passes_required_checks() => "passed".to_owned(),
            Ok(evidence) if evidence.waiver_by.is_some() => {
                format!("waived by {}", evidence.waiver_by.unwrap_or_default())
            }
            Ok(_) => "failed".to_owned(),
            Err(_) => "not run".to_owned(),
        };
    let agent = match (&session.agent_command, session.agent_exit_code) {
        (Some(command), Some(code)) => format!("{command} (exit: {code})"),
        (Some(command), None) => format!("{command} (exit: unavailable)"),
        (None, _) => "not run".to_owned(),
    };
    let acp_session = match (&session.acp_agent, &session.acp_session_id) {
        (Some(agent), Some(session_id)) => format!("{agent} ({session_id})"),
        _ => "none".to_owned(),
    };
    format!(
        "issue #{}\nstate: {}\nbranch: {}\ncurrent branch: {}\nrequired checks: {}\nagent: {}\nACP session: {}\nblocker: {}\nblocker owner: {}\nnext action: {}",
        session.issue_number,
        session.state.as_str(),
        session.branch,
        current_branch,
        required_checks,
        agent,
        acp_session,
        blocker,
        blocker_owner,
        next_action(session.state)
    )
}

fn next_action(state: IssueState) -> &'static str {
    match state {
        IssueState::NeedsSpec => "complete the issue contract",
        IssueState::SpecReady => "obtain approval for ready-for-build",
        IssueState::ReadyForBuild => "start an implementation session",
        IssueState::InProgress => "run verification and prepare review",
        IssueState::InReview => "review evidence and decide whether to close",
        IssueState::Done => "no action; issue is complete",
        IssueState::Blocked => "resolve the recorded blocker",
    }
}

fn issue(arguments: &[String], directory: &Path) -> Result<String, String> {
    match arguments {
        [command, number, options @ ..] if command == "validate" => {
            validate_issue(number, options, directory)
        }
        [command, path] if command == "scaffold" => scaffold_issue(path, directory),
        [command, options @ ..] if command == "create" => create_issue(options, directory),
        [command, number, options @ ..] if command == "adapt" => {
            adapt_issue(number, options, directory)
        }
        _ => Err("Usage: conveyor issue <validate|scaffold|create|adapt> ...".to_owned()),
    }
}

fn scaffold_issue(path: &str, directory: &Path) -> Result<String, String> {
    let path = directory.join(path);
    if path.exists() {
        return Err(format!("refusing to overwrite {}", path.display()));
    }
    let parent = path.parent().ok_or("issue template path has no parent")?;
    std::fs::create_dir_all(parent)
        .map_err(|error| format!("could not create {}: {error}", parent.display()))?;
    std::fs::write(&path, issue_template())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(format!(
        "created Conveyor issue scaffold: {}",
        path.display()
    ))
}

fn create_issue(arguments: &[String], directory: &Path) -> Result<String, String> {
    let mut title = None;
    let mut body_file = None;
    let mut explicit_yes = false;
    let mut index = 0;
    while index < arguments.len() {
        match arguments[index].as_str() {
            "--title" => {
                title = arguments.get(index + 1).cloned();
                index += 2;
            }
            "--body-file" => {
                body_file = arguments.get(index + 1).cloned();
                index += 2;
            }
            "--yes" => {
                explicit_yes = true;
                index += 1;
            }
            option => return Err(format!("unknown issue create option: {option}")),
        }
    }
    let title = title
        .filter(|value| !value.trim().is_empty())
        .ok_or("Usage: conveyor issue create --title <title> --body-file <path> [--yes]")?;
    let body_file = body_file
        .ok_or("Usage: conveyor issue create --title <title> --body-file <path> [--yes]")?;
    let body_path = directory.join(&body_file);
    let body = std::fs::read_to_string(&body_path)
        .map_err(|error| format!("could not read {}: {error}", body_path.display()))?;
    let report = validate(&from_markdown(&body));
    if !report.is_valid() {
        return Err(format!(
            "issue body is not Conveyor-ready; missing:\n{}",
            report
                .missing
                .iter()
                .map(|item| format!("- {}", item.label()))
                .collect::<Vec<_>>()
                .join("\n")
        ));
    }
    let configuration = load_config(directory)?;
    let summary = format!(
        "Create GitHub issue in {}\nTitle: {title}\nBody: {}",
        configuration.github.repository,
        body_path.display()
    );
    if authorize(explicit_yes, &TerminalPrompt, &summary) == Decision::Declined {
        return Err("issue creation cancelled; no GitHub issue was created".to_owned());
    }
    let output = Command::new("gh")
        .args([
            "issue",
            "create",
            "--repo",
            &configuration.github.repository,
            "--title",
            &title,
            "--body-file",
            &body_file,
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not create issue: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(format!(
        "created Conveyor-ready issue: {}",
        String::from_utf8_lossy(&output.stdout).trim()
    ))
}

fn adapt_issue(number: &str, arguments: &[String], directory: &Path) -> Result<String, String> {
    let issue_number = parse_issue_number(number)?;
    let (apply, explicit_yes) = match arguments {
        [] => (false, false),
        [flag] if flag == "--apply" => (true, false),
        [flag] if flag == "--yes" => (false, true),
        [apply, yes] if apply == "--apply" && yes == "--yes" => (true, true),
        _ => return Err("Usage: conveyor issue adapt <number> [--apply] [--yes]".to_owned()),
    };
    let configuration = load_config(directory)?;
    let source = fetch_issue_body(number, directory)?;
    let agent_name = &configuration.agents.planner;
    let agent = configuration
        .acp_agent(agent_name)
        .ok_or_else(|| format!("planner `{agent_name}` must be an ACP agent to adapt an issue"))?;
    let prompt = format!(
        "Rewrite the GitHub issue below as a complete Conveyor issue contract. Preserve factual intent; do not invent requirements. Return only Markdown using exactly this structure:\n\n{}\n\nExisting issue:\n{}",
        issue_template(),
        source
    );
    #[cfg(feature = "acp")]
    let proposal = run_acp_agent(&agent, &prompt, directory, None)?.response;
    #[cfg(not(feature = "acp"))]
    {
        let _ = (agent, prompt, issue_number, apply, explicit_yes);
        Err(acp_feature_required(agent_name))
    }
    #[cfg(feature = "acp")]
    {
        let report = validate(&from_markdown(&proposal));
        if !report.is_valid() {
            return Err(format!(
                "planner produced an incomplete adaptation; missing:\n{}",
                report
                    .missing
                    .iter()
                    .map(|item| format!("- {}", item.label()))
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        let proposals = directory.join(".conveyor/issues");
        std::fs::create_dir_all(&proposals)
            .map_err(|error| format!("could not create {}: {error}", proposals.display()))?;
        let proposal_path = proposals.join(format!("{issue_number}-adapted.md"));
        std::fs::write(&proposal_path, &proposal)
            .map_err(|error| format!("could not write {}: {error}", proposal_path.display()))?;
        if !apply {
            return Ok(format!(
                "created validated adaptation: {}\nReview it, then apply with `conveyor issue adapt {issue_number} --apply`",
                proposal_path.display()
            ));
        }
        let summary = format!(
            "Replace GitHub issue #{issue_number} body with validated proposal {}",
            proposal_path.display()
        );
        if authorize(explicit_yes, &TerminalPrompt, &summary) == Decision::Declined {
            return Ok(format!(
                "created validated adaptation: {}; GitHub issue was not changed",
                proposal_path.display()
            ));
        }
        run_command(
            "gh",
            [
                "issue",
                "edit",
                number,
                "--repo",
                &configuration.github.repository,
                "--body-file",
                proposal_path
                    .to_str()
                    .ok_or("proposal path is not valid UTF-8")?,
            ],
            directory,
            "could not update adapted issue",
        )?;
        record_audit(
            directory,
            issue_number,
            "adapt_issue",
            proposal_path.display().to_string(),
        )?;
        Ok(format!(
            "updated issue #{issue_number} from {}",
            proposal_path.display()
        ))
    }
}

fn validate_issue(number: &str, options: &[String], directory: &Path) -> Result<String, String> {
    let issue_number = parse_issue_number(number)?;
    let format = parse_format(options)?;
    let report = validate(&from_markdown(&fetch_issue_body(number, directory)?));
    match format {
        ReportFormat::Human if report.is_valid() => {
            Ok(format!("issue #{issue_number}: ready for build"))
        }
        ReportFormat::Human => Ok(format!(
            "issue #{issue_number}: not ready for build\nmissing:\n{}",
            report
                .missing
                .iter()
                .map(|requirement| format!("- {}", requirement.label()))
                .collect::<Vec<_>>()
                .join("\n")
        )),
        ReportFormat::Json => Ok(format!(
            "{{\"issue\":{issue_number},\"valid\":{},\"missing\":[{}]}}",
            report.is_valid(),
            report
                .missing
                .iter()
                .map(|requirement| format!("\"{}\"", requirement.label()))
                .collect::<Vec<_>>()
                .join(",")
        )),
    }
}

fn fetch_issue_body(number: &str, directory: &Path) -> Result<String, String> {
    let configuration = load_config(directory)?;
    let output = Command::new("gh")
        .args([
            "issue",
            "view",
            number,
            "--repo",
            &configuration.github.repository,
            "--json",
            "body",
            "--jq",
            ".body",
        ])
        .current_dir(directory)
        .output()
        .map_err(|error| format!("could not run gh: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "could not fetch issue #{number}: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

fn parse_issue_number(number: &str) -> Result<u64, String> {
    let issue_number = number
        .parse::<u64>()
        .map_err(|_| format!("issue number must be a positive integer: {number}"))?;
    (issue_number > 0)
        .then_some(issue_number)
        .ok_or_else(|| "issue number must be a positive integer: 0".to_owned())
}

#[derive(Clone, Copy)]
enum ReportFormat {
    Human,
    Json,
}

fn parse_format(options: &[String]) -> Result<ReportFormat, String> {
    match options {
        [] => Ok(ReportFormat::Human),
        [option, value] if option == "--format" && value == "human" => Ok(ReportFormat::Human),
        [option, value] if option == "--format" && value == "json" => Ok(ReportFormat::Json),
        _ => Err("expected --format human or --format json".to_owned()),
    }
}

fn parse_verify_arguments(options: &[String]) -> Result<(ReportFormat, bool), String> {
    let mut format = ReportFormat::Human;
    let mut explicit_yes = false;
    let mut index = 0;
    while index < options.len() {
        match options[index].as_str() {
            "--yes" if !explicit_yes => explicit_yes = true,
            "--format" => {
                let Some(value) = options.get(index + 1) else {
                    return Err("Usage: conveyor verify [--format human|json] [--yes]".to_owned());
                };
                format = match value.as_str() {
                    "human" => ReportFormat::Human,
                    "json" => ReportFormat::Json,
                    _ => {
                        return Err(
                            "Usage: conveyor verify [--format human|json] [--yes]".to_owned()
                        );
                    }
                };
                index += 1;
            }
            _ => return Err("Usage: conveyor verify [--format human|json] [--yes]".to_owned()),
        }
        index += 1;
    }
    Ok((format, explicit_yes))
}

fn doctor(directory: &Path) -> Result<String, String> {
    let configuration = load_config(directory)?;
    let report = Doctor::new(&SystemProbe { directory }).check(&configuration);
    if report.is_healthy() {
        return Ok("doctor: all checks passed".to_owned());
    }
    let findings = report
        .findings
        .iter()
        .map(|finding| format!("{}: {}", finding_name(finding.kind), finding.detail))
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "doctor: {} issue(s) found\n{findings}",
        report.findings.len()
    ))
}

fn finding_name(kind: FindingKind) -> &'static str {
    match kind {
        FindingKind::GitRepository => "git repository",
        FindingKind::GitRemote => "git remote",
        FindingKind::GithubAuthentication => "github authentication",
        FindingKind::WorkflowLabel => "workflow label",
        FindingKind::AgentExecutable => "agent executable",
    }
}

struct SystemProbe<'a> {
    directory: &'a Path,
}

impl Probe for SystemProbe<'_> {
    fn git_repository(&self) -> ProbeResult {
        command_succeeds(
            "git",
            ["rev-parse", "--is-inside-work-tree"],
            Some(self.directory),
        )
        .map_or_else(
            || ProbeResult::Missing("not inside a Git worktree".to_owned()),
            |_| ProbeResult::Available,
        )
    }

    fn git_remote(&self, name: &str) -> ProbeResult {
        command_succeeds("git", ["remote", "get-url", name], Some(self.directory)).map_or_else(
            || ProbeResult::Missing(format!("configured remote `{name}` does not exist")),
            |_| ProbeResult::Available,
        )
    }

    fn github_authentication(&self) -> ProbeResult {
        command_succeeds("gh", ["auth", "status"], Some(self.directory)).map_or_else(
            || ProbeResult::Missing("run `gh auth login` to authenticate".to_owned()),
            |_| ProbeResult::Available,
        )
    }

    fn github_label(&self, repository: &str, label: &str) -> ProbeResult {
        let output = Command::new("gh")
            .args([
                "label", "list", "--repo", repository, "--limit", "1000", "--json", "name", "--jq",
                ".[].name",
            ])
            .current_dir(self.directory)
            .output();
        match output {
            Ok(output)
                if output.status.success()
                    && String::from_utf8_lossy(&output.stdout)
                        .lines()
                        .any(|name| name == label) =>
            {
                ProbeResult::Available
            }
            Ok(output) if output.status.success() => ProbeResult::Missing(format!(
                "configured label `{label}` does not exist in {repository}"
            )),
            Ok(output) => ProbeResult::Missing(format!(
                "could not list labels in {repository}: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            )),
            Err(error) => ProbeResult::Missing(format!(
                "could not run gh to inspect labels in {repository}: {error}"
            )),
        }
    }

    fn executable(&self, name: &str) -> ProbeResult {
        if executable_on_path(name) {
            ProbeResult::Available
        } else {
            ProbeResult::Missing(format!("{name} is not on PATH"))
        }
    }
}

fn command_succeeds<const N: usize>(
    program: &str,
    arguments: [&str; N],
    directory: Option<&Path>,
) -> Option<()> {
    let mut command = Command::new(program);
    command
        .args(arguments)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Some(directory) = directory {
        command.current_dir(directory);
    }
    command.status().ok()?.success().then_some(())
}

fn executable_on_path(name: &str) -> bool {
    std::env::var_os("PATH").is_some_and(|paths| {
        std::env::split_paths(&paths).any(|directory| directory.join(name).is_file())
    })
}

fn initialize(arguments: &[String], directory: &Path) -> Result<String, String> {
    let mut repository = None;
    let mut target_branch = "main".to_owned();
    let mut index = 0;
    while index < arguments.len() {
        let value = arguments
            .get(index + 1)
            .ok_or_else(|| format!("missing value for {}", arguments[index]))?;
        match arguments[index].as_str() {
            "--repository" => repository = Some(value.to_owned()),
            "--target-branch" => target_branch = value.to_owned(),
            option => return Err(format!("unknown init option: {option}")),
        }
        index += 2;
    }
    let repository = repository.ok_or("init requires --repository <owner/repository>")?;
    let path = directory.join(".conveyor.toml");
    let content = Config::new(repository, target_branch)
        .to_toml()
        .map_err(|error| error.to_string())?;
    let mut output = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                format!(
                    "{} already exists; refusing to overwrite it",
                    path.display()
                )
            } else {
                format!("could not create {}: {error}", path.display())
            }
        })?;
    output
        .write_all(content.as_bytes())
        .map_err(|error| format!("could not write {}: {error}", path.display()))?;
    Ok(format!("created {}", path.display()))
}

fn usage() -> &'static str {
    "Usage:\n  conveyor init --repository <owner/repository> [--target-branch <branch>]\n  conveyor doctor\n  conveyor status [issue]\n  conveyor block <number> --reason <text> --owner <name>\n  conveyor resume <number> --to <state>\n  conveyor agent continue <number> <prompt>\n  conveyor issue scaffold <path>\n  conveyor issue create --title <title> --body-file <path> [--yes]\n  conveyor issue adapt <number> [--apply] [--yes]\n  conveyor issue validate <number> [--format human|json]\n  conveyor handoff <number> --role <planner|implementer|reviewer>\n  conveyor build <number> [--agent <name>] [--dry-run|--yes]\n  conveyor build retry <number> [--yes]\n  conveyor verify [--format human|json] [--yes]\n  conveyor review <number> [--agent <name>]\n  conveyor pr open <number> [--waive-by <name>] [--yes]\n  conveyor audit <number>"
}

#[cfg(test)]
mod tests {
    use std::fs;

    use conveyor_core::config::{Check, Config};
    use conveyor_core::session::{SessionRecord, SessionStore};
    use conveyor_core::task_packet::{AgentRole, TaskPacket};
    use conveyor_core::workflow::IssueState;

    use super::{
        evidence_path, parse_build_arguments, parse_review_arguments, parse_verify_arguments, run,
        write_packet,
    };

    fn temporary_directory(name: &str) -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!("conveyor-cli-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        fs::create_dir_all(&path).unwrap();
        path
    }

    #[test]
    fn init_writes_a_safe_configuration() {
        let directory = temporary_directory("init-writes");

        run(
            [
                "init",
                "--repository",
                "octo-org/conveyor",
                "--target-branch",
                "trunk",
            ],
            &directory,
        )
        .unwrap();

        let configuration = fs::read_to_string(directory.join(".conveyor.toml")).unwrap();
        assert!(configuration.contains("repository = \"octo-org/conveyor\""));
        assert!(configuration.contains("target_branch = \"trunk\""));
        assert!(configuration.contains("draft_prs_only = true"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn issue_scaffold_writes_a_valid_contract_template() {
        let directory = temporary_directory("issue-scaffold");

        let output = run(["issue", "scaffold", "docs/issues/example.md"], &directory).unwrap();
        let path = directory.join("docs/issues/example.md");

        assert!(output.contains("created Conveyor issue scaffold"));
        assert!(path.is_file());
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("## Acceptance criteria")
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn build_arguments_accept_an_agent_and_yes_in_either_order() {
        let arguments = [
            "70".to_owned(),
            "--yes".to_owned(),
            "--agent".to_owned(),
            "local".to_owned(),
        ];
        let parsed = parse_build_arguments(&arguments).unwrap();

        assert_eq!(parsed, ("70", true, false, Some("local")));
        assert!(parse_build_arguments(&["70".to_owned(), "--agent".to_owned()]).is_err());
        assert_eq!(
            parse_build_arguments(&["70".to_owned(), "--dry-run".to_owned()]).unwrap(),
            ("70", false, true, None)
        );
        assert!(parse_build_arguments(&["70".to_owned(), "--open-draft-pr".to_owned()]).is_err());
        assert!(
            parse_build_arguments(&["70".to_owned(), "--dry-run".to_owned(), "--yes".to_owned(),])
                .is_err()
        );
    }

    #[test]
    fn verify_arguments_allow_non_interactive_pr_comments() {
        assert!(matches!(
            parse_verify_arguments(&["--yes".to_owned(), "--format".to_owned(), "json".to_owned()]),
            Ok((super::ReportFormat::Json, true))
        ));
    }

    #[test]
    fn review_arguments_optionally_select_a_configured_agent() {
        assert_eq!(
            parse_review_arguments(&[
                "70".to_owned(),
                "--agent".to_owned(),
                "local-reviewer".to_owned(),
            ])
            .unwrap(),
            ("70", Some("local-reviewer"))
        );
    }

    #[test]
    fn block_and_resume_update_a_local_session_with_an_owner() {
        let directory = temporary_directory("block-resume");
        let store = SessionStore::new(&directory);
        store
            .save(&SessionRecord::new(70, IssueState::InProgress, "feat/70"))
            .unwrap();

        let blocked = run(
            [
                "block",
                "70",
                "--reason",
                "waiting for access",
                "--owner",
                "platform",
            ],
            &directory,
        )
        .unwrap();
        assert!(blocked.contains("owner: platform"));
        let status = run(["status", "70"], &directory).unwrap();
        assert!(status.contains("state: blocked"));
        assert!(status.contains("blocker owner: platform"));

        run(["resume", "70", "--to", "in-progress"], &directory).unwrap();
        assert_eq!(
            store.load(70).unwrap().unwrap().state,
            IssueState::InProgress
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn init_never_overwrites_an_existing_configuration() {
        let directory = temporary_directory("init-no-overwrite");
        let path = directory.join(".conveyor.toml");
        fs::write(&path, "preserve this file").unwrap();

        let error = run(["init", "--repository", "octo-org/conveyor"], &directory).unwrap_err();

        assert!(error.contains("already exists"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "preserve this file");
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn status_explains_when_no_local_session_exists() {
        let directory = temporary_directory("status-empty");

        assert_eq!(
            run(["status"], &directory).unwrap(),
            "status: no active local sessions"
        );
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn status_shows_the_requested_session_and_next_action() {
        let directory = temporary_directory("status-session");
        let session = SessionRecord::new(70, IssueState::InProgress, "feat/70-readiness");
        SessionStore::new(&directory).save(&session).unwrap();

        let output = run(["status", "70"], &directory).unwrap();

        assert!(output.contains("issue #70"));
        assert!(output.contains("state: in-progress"));
        assert!(output.contains("branch: feat/70-readiness"));
        assert!(output.contains("current branch: unknown"));
        assert!(output.contains("required checks: not run"));
        assert!(output.contains("agent: not run"));
        assert!(output.contains("next action: run verification and prepare review"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn status_marks_evidence_from_another_revision_as_stale() {
        let directory = temporary_directory("status-stale-evidence");
        fs::write(
            directory.join(".conveyor.toml"),
            Config::new("octo-org/conveyor", "main").to_toml().unwrap(),
        )
        .unwrap();
        let store = SessionStore::new(&directory);
        store
            .save(&SessionRecord::new(70, IssueState::InProgress, "feat/70"))
            .unwrap();
        fs::create_dir_all(directory.join(".conveyor/reports")).unwrap();
        fs::write(
            directory.join(".conveyor/reports/latest.json"),
            "{\"revision\":\"old\",\"waiver_by\":null,\"passed_required_checks\":true,\"checks\":[]}",
        )
        .unwrap();

        let output = run(["status", "70"], &directory).unwrap();

        assert!(output.contains("required checks: stale; run verify"));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn evidence_path_cannot_escape_the_repository() {
        let directory = temporary_directory("evidence-path");
        let mut configuration = Config::new("octo-org/conveyor", "main");
        configuration.conveyor.verification_report_path = "../outside.json".to_owned();

        assert!(evidence_path(&directory, &configuration).is_err());
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn verify_runs_configured_checks_and_reports_a_pass() {
        let directory = temporary_directory("verify");
        let mut configuration = Config::new("octo-org/conveyor", "main");
        configuration.conveyor.verification_report_path =
            ".conveyor/evidence/current.json".to_owned();
        configuration.checks.push(Check {
            name: "always passes".to_owned(),
            command: "true".to_owned(),
            required: true,
        });
        fs::write(
            directory.join(".conveyor.toml"),
            configuration.to_toml().unwrap(),
        )
        .unwrap();

        let output = run(["verify"], &directory).unwrap();

        assert!(output.contains("verification: passed"));
        assert!(output.contains("always passes: passed"));
        let evidence =
            fs::read_to_string(directory.join(".conveyor/evidence/current.json")).unwrap();
        assert!(evidence.contains("\"passed_required_checks\":true"));
        assert!(evidence.contains("\"command\":\"true\""));
        fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn writes_a_handoff_packet_to_local_conveyor_storage() {
        let directory = temporary_directory("packet");
        let packet = TaskPacket::new(
            70,
            AgentRole::Implementer,
            "feat/70-handoff",
            "feat: issue #70",
            conveyor_core::issue_contract::IssueContract {
                problem: "Make handoffs portable.".to_owned(),
                in_scope: "Task packet creation.".to_owned(),
                out_of_scope: "Agent execution.".to_owned(),
                acceptance_criteria: "- Packet is written.".to_owned(),
                affected_components: "conveyor-cli".to_owned(),
                verification_plan: "cargo test --workspace".to_owned(),
                dependencies_risks: "None.".to_owned(),
                open_questions: "None.".to_owned(),
                delivery_slice: None,
            },
            "Use TDD.",
            Vec::new(),
        );

        let path = write_packet(&directory, &packet).unwrap();

        assert_eq!(path, directory.join(".conveyor/packets/70-implementer.md"));
        assert!(
            fs::read_to_string(path)
                .unwrap()
                .contains("Make handoffs portable.")
        );
        fs::remove_dir_all(directory).unwrap();
    }
}
