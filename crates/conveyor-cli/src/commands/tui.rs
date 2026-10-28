//! Interactive operational console for the existing Conveyor command surface.

use std::collections::BTreeMap;
use std::io::{self, stdout};
use std::path::Path;
use std::time::{Duration, Instant};

use clap::Parser;
use conveyor_core::session::{SessionRecord, SessionStore};
use crossterm::event::{self, Event, KeyCode, KeyEventKind};
use crossterm::execute;
use crossterm::terminal::{
    EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode,
};
use ratatui::Terminal;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Widget, Wrap};

/// Starts the local console and dispatches workflow commands through the normal CLI path.
pub(crate) fn run(directory: &Path) -> Result<String, String> {
    let mut app = ConsoleApp::load(directory)?;
    loop {
        let action = run_console_screen(&mut app, directory)?;
        match action {
            ConsoleAction::Quit => return Ok("Conveyor console closed.".to_owned()),
            ConsoleAction::Run(command) => {
                let message = execute_existing_command(&command, directory);
                println!("\n{message}\n\nPress Enter to return to Conveyor...");
                let mut input = String::new();
                io::stdin()
                    .read_line(&mut input)
                    .map_err(|error| format!("could not read return prompt: {error}"))?;
                app.reload(directory)?;
            }
            ConsoleAction::Reload => app.reload(directory)?,
            ConsoleAction::None => unreachable!("the console loop never returns no action"),
        }
    }
}

struct ConsoleTerminal {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    restored: bool,
}

impl ConsoleTerminal {
    fn start() -> Result<Self, String> {
        enable_raw_mode().map_err(|error| format!("could not enable console input: {error}"))?;
        let mut output = stdout();
        if let Err(error) = execute!(output, EnterAlternateScreen) {
            let _ = disable_raw_mode();
            return Err(format!("could not open console screen: {error}"));
        }
        let terminal = Terminal::new(CrosstermBackend::new(output))
            .map_err(|error| format!("could not create console: {error}"))?;
        Ok(Self {
            terminal,
            restored: false,
        })
    }

    fn restore(&mut self) -> Result<(), String> {
        if self.restored {
            return Ok(());
        }
        self.restored = true;
        disable_raw_mode().map_err(|error| format!("could not restore terminal input: {error}"))?;
        execute!(self.terminal.backend_mut(), LeaveAlternateScreen)
            .map_err(|error| format!("could not restore terminal screen: {error}"))?;
        self.terminal
            .show_cursor()
            .map_err(|error| format!("could not restore terminal cursor: {error}"))
    }
}

impl Drop for ConsoleTerminal {
    fn drop(&mut self) {
        let _ = self.restore();
    }
}

fn run_console_screen(app: &mut ConsoleApp, directory: &Path) -> Result<ConsoleAction, String> {
    let mut console = ConsoleTerminal::start()?;
    let mut last_refresh = Instant::now();
    loop {
        console
            .terminal
            .draw(|frame| app.draw(frame.area(), frame.buffer_mut()))
            .map_err(|error| format!("could not draw console: {error}"))?;
        if !event::poll(Duration::from_millis(250))
            .map_err(|error| format!("could not read console input: {error}"))?
        {
            if last_refresh.elapsed() >= Duration::from_secs(1) {
                app.reload(directory)?;
                last_refresh = Instant::now();
            }
            continue;
        }
        let Event::Key(key) =
            event::read().map_err(|error| format!("could not read key: {error}"))?
        else {
            continue;
        };
        if key.kind != KeyEventKind::Press {
            continue;
        }
        let action = app.handle_key(key.code);
        match action {
            ConsoleAction::None => {}
            ConsoleAction::Quit | ConsoleAction::Run(_) => return Ok(action),
            ConsoleAction::Reload => app.reload(directory)?,
        }
    }
}

fn execute_existing_command(command: &str, directory: &Path) -> String {
    let arguments = match tokenize_command(command) {
        Ok(arguments) if !arguments.is_empty() => arguments,
        Ok(_) => return "No Conveyor command was entered.".to_owned(),
        Err(error) => return error,
    };
    let invocation = std::iter::once("conveyor".to_owned()).chain(arguments);
    match crate::cli::Cli::try_parse_from(invocation) {
        Ok(cli) => crate::commands::execute(cli, directory)
            .unwrap_or_else(|error| format!("conveyor: {error}")),
        Err(error) => error.to_string(),
    }
}

/// Splits command-palette input into process-style arguments without invoking a shell.
fn tokenize_command(input: &str) -> Result<Vec<String>, String> {
    let mut arguments = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    for character in input.chars() {
        if escaped {
            current.push(character);
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if Some(character) == quote {
            quote = None;
        } else if matches!(character, '\'' | '"') && quote.is_none() {
            quote = Some(character);
        } else if character.is_whitespace() && quote.is_none() {
            if !current.is_empty() {
                arguments.push(std::mem::take(&mut current));
            }
        } else {
            current.push(character);
        }
    }
    if escaped || quote.is_some() {
        return Err("command has an unfinished quote or escape".to_owned());
    }
    if !current.is_empty() {
        arguments.push(current);
    }
    Ok(arguments)
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConsoleCommand {
    Help,
    Open(u64),
    Reload,
    Quit,
    Run(String),
    Invalid(String),
}

impl ConsoleCommand {
    fn parse(input: &str) -> Self {
        let mut parts = input.split_whitespace();
        match (parts.next(), parts.next(), parts.next()) {
            (Some("/help"), None, None) => Self::Help,
            (Some("/status"), None, None) | (Some("/reload"), None, None) => Self::Reload,
            (Some("/quit" | "/q"), None, None) => Self::Quit,
            (Some("/open"), Some(issue), None) => issue
                .parse()
                .map(Self::Open)
                .unwrap_or_else(|_| Self::Invalid("usage: /open <issue-number>".to_owned())),
            _ if input.starts_with('/') => Self::Run(input.trim_start_matches('/').to_owned()),
            _ => Self::Invalid("commands begin with /".to_owned()),
        }
    }
}

#[derive(Clone, Debug)]
struct ConsoleModel {
    issues: Vec<u64>,
    selected: usize,
    expanded: bool,
    status: String,
}

impl ConsoleModel {
    fn new(issues: Vec<u64>) -> Self {
        Self {
            issues,
            selected: 0,
            expanded: false,
            status: "Ready. Press / for commands.".to_owned(),
        }
    }

    fn selected_issue(&self) -> Option<u64> {
        self.issues.get(self.selected).copied()
    }

    #[cfg(test)]
    fn is_expanded(&self) -> bool {
        self.expanded
    }

    fn submit_command(&mut self, input: &str) -> ConsoleAction {
        match ConsoleCommand::parse(input) {
            ConsoleCommand::Help => {
                self.status = "Use any Conveyor command: /build 70, /issue scaffold <path>, /status. /open <issue>, /quit.".to_owned();
                ConsoleAction::None
            }
            ConsoleCommand::Reload => ConsoleAction::Reload,
            ConsoleCommand::Quit => ConsoleAction::Quit,
            ConsoleCommand::Run(command) => ConsoleAction::Run(command),
            ConsoleCommand::Open(issue) => {
                match self.issues.iter().position(|current| *current == issue) {
                    Some(index) => {
                        self.selected = index;
                        self.expanded = true;
                        self.status = format!("Opened issue #{issue}.");
                        ConsoleAction::None
                    }
                    None => {
                        self.status = format!("No local session for issue #{issue}.");
                        ConsoleAction::None
                    }
                }
            }
            ConsoleCommand::Invalid(message) => {
                self.status = message;
                ConsoleAction::None
            }
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum ConsoleAction {
    None,
    Reload,
    Quit,
    Run(String),
}

struct ConsoleApp {
    model: ConsoleModel,
    sessions: Vec<SessionRecord>,
    audit_summaries: BTreeMap<u64, Vec<String>>,
    command_input: Option<String>,
}

impl ConsoleApp {
    fn load(directory: &Path) -> Result<Self, String> {
        let sessions = SessionStore::new(directory)
            .list()
            .map_err(|error| error.to_string())?;
        let issues = sessions
            .iter()
            .map(|session| session.issue_number)
            .collect();
        Ok(Self {
            model: ConsoleModel::new(issues),
            audit_summaries: load_audit_summaries(directory, &sessions),
            sessions,
            command_input: None,
        })
    }

    fn reload(&mut self, directory: &Path) -> Result<(), String> {
        let selected = self.model.selected_issue();
        let expanded = self.model.expanded;
        let sessions = SessionStore::new(directory)
            .list()
            .map_err(|error| error.to_string())?;
        self.model = ConsoleModel::new(
            sessions
                .iter()
                .map(|session| session.issue_number)
                .collect(),
        );
        if let Some(issue) = selected.and_then(|issue| {
            self.model
                .issues
                .iter()
                .position(|current| *current == issue)
        }) {
            self.model.selected = issue;
            self.model.expanded = expanded;
        }
        self.model.status = "Reloaded local sessions.".to_owned();
        self.audit_summaries = load_audit_summaries(directory, &sessions);
        self.sessions = sessions;
        Ok(())
    }

    fn handle_key(&mut self, key: KeyCode) -> ConsoleAction {
        if let Some(input) = &mut self.command_input {
            match key {
                KeyCode::Esc => self.command_input = None,
                KeyCode::Enter => {
                    let command = std::mem::take(input);
                    self.command_input = None;
                    return self.model.submit_command(&command);
                }
                KeyCode::Backspace => {
                    input.pop();
                }
                KeyCode::Char(character) => input.push(character),
                _ => {}
            }
            return ConsoleAction::None;
        }
        match key {
            KeyCode::Char('q') => ConsoleAction::Quit,
            KeyCode::Char('r') => ConsoleAction::Reload,
            KeyCode::Char('/') => {
                self.command_input = Some("/".to_owned());
                ConsoleAction::None
            }
            KeyCode::Down | KeyCode::Char('j') => {
                self.model.selected =
                    (self.model.selected + 1).min(self.sessions.len().saturating_sub(1));
                ConsoleAction::None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.model.selected = self.model.selected.saturating_sub(1);
                ConsoleAction::None
            }
            KeyCode::Enter | KeyCode::Char(' ') => {
                self.model.expanded = !self.model.expanded;
                ConsoleAction::None
            }
            _ => ConsoleAction::None,
        }
    }

    fn draw(&self, area: Rect, buffer: &mut ratatui::buffer::Buffer) {
        let rows = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3),
                Constraint::Min(8),
                Constraint::Length(3),
            ])
            .split(area);
        Paragraph::new(Line::from(vec![
            Span::styled(
                " Conveyor ",
                Style::default()
                    .fg(Color::Black)
                    .bg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" local workflow console"),
        ]))
        .block(Block::default().borders(Borders::ALL))
        .render(rows[0], buffer);
        let columns = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(38), Constraint::Percentage(62)])
            .split(rows[1]);
        let items = self
            .sessions
            .iter()
            .enumerate()
            .map(|(index, session)| {
                let marker = if index == self.model.selected {
                    "›"
                } else {
                    " "
                };
                let detail = session.blocker.as_deref().unwrap_or(&session.branch);
                ListItem::new(format!(
                    "{marker} #{:<5} {:<12} {detail}",
                    session.issue_number,
                    session.state.as_str()
                ))
            })
            .collect::<Vec<_>>();
        List::new(items)
            .block(Block::default().title(" Sessions ").borders(Borders::ALL))
            .highlight_style(Style::default().bg(Color::DarkGray))
            .render(columns[0], buffer);
        self.details()
            .block(Block::default().title(" Details ").borders(Borders::ALL))
            .wrap(Wrap { trim: false })
            .render(columns[1], buffer);
        let footer = self.command_input.as_deref().map_or_else(
            || {
                format!(
                    "{}  |  j/k select • enter expand • / commands • r reload • q quit",
                    self.model.status
                )
            },
            |input| format!("Command: {input}"),
        );
        Paragraph::new(footer)
            .block(Block::default().borders(Borders::ALL))
            .render(rows[2], buffer);
    }

    fn details(&self) -> Paragraph<'static> {
        let Some(session) = self.sessions.get(self.model.selected) else {
            return Paragraph::new(
                "No local sessions. Run a Conveyor workflow command to create one.",
            );
        };
        let mut text = format!(
            "Issue #{}\nState: {}\nBranch: {}",
            session.issue_number,
            session.state.as_str(),
            session.branch
        );
        if self.model.expanded {
            text.push_str(&format!(
                "\n\nAgent: {}\nACP session: {}\nBlocker: {}\nOwner: {}",
                session.agent_command.as_deref().unwrap_or("not recorded"),
                session.acp_session_id.as_deref().unwrap_or("not recorded"),
                session.blocker.as_deref().unwrap_or("none"),
                session.blocker_owner.as_deref().unwrap_or("none")
            ));
            if let Some(events) = self.audit_summaries.get(&session.issue_number) {
                text.push_str("\n\nRecent audit events:");
                for event in events {
                    text.push_str(&format!("\n{event}"));
                }
            }
        } else {
            text.push_str("\n\nPress Enter to expand session details.");
        }
        Paragraph::new(text)
    }
}

fn load_audit_summaries(
    directory: &Path,
    sessions: &[SessionRecord],
) -> BTreeMap<u64, Vec<String>> {
    sessions
        .iter()
        .filter_map(|session| {
            let audit_path =
                directory.join(format!(".conveyor/audit/{}.jsonl", session.issue_number));
            let event_path =
                directory.join(format!(".conveyor/events/{}.jsonl", session.issue_number));
            let events = [audit_path, event_path]
                .into_iter()
                .filter_map(|path| std::fs::read_to_string(path).ok())
                .collect::<Vec<_>>()
                .join("\n");
            if events.is_empty() {
                return None;
            }
            let lines = events
                .lines()
                .rev()
                .take(5)
                .map(summarize_audit_event)
                .collect::<Vec<_>>();
            (!lines.is_empty()).then_some((session.issue_number, lines))
        })
        .collect()
}

fn summarize_audit_event(event: &str) -> String {
    if let (Some(category), Some(summary)) = (
        audit_json_string(event, "category"),
        audit_json_string(event, "summary"),
    ) {
        return format!("{category}: {summary}");
    }
    match (
        audit_json_string(event, "action"),
        audit_json_string(event, "target"),
    ) {
        (Some(action), Some(target)) => format!("{action}: {target}"),
        _ => event.to_owned(),
    }
}

fn audit_json_string(event: &str, key: &str) -> Option<String> {
    let prefix = format!("\"{key}\":\"");
    let rest = event.split_once(&prefix)?.1;
    let mut value = String::new();
    let mut escaped = false;
    for character in rest.chars() {
        if escaped {
            value.push(match character {
                'n' => '\n',
                'r' => '\r',
                't' => '\t',
                other => other,
            });
            escaped = false;
        } else if character == '\\' {
            escaped = true;
        } else if character == '"' {
            return Some(value);
        } else {
            value.push(character);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::{ConsoleCommand, ConsoleModel, tokenize_command};

    #[test]
    fn slash_open_selects_and_expands_the_requested_issue() {
        let mut model = ConsoleModel::new(vec![12, 70]);

        model.submit_command("/open 70");

        assert_eq!(model.selected_issue(), Some(70));
        assert!(model.is_expanded());
    }

    #[test]
    fn slash_help_is_non_destructive() {
        assert_eq!(ConsoleCommand::parse("/help"), ConsoleCommand::Help);
    }

    #[test]
    fn workflow_commands_are_forwarded_to_the_existing_cli() {
        assert_eq!(
            ConsoleCommand::parse("/issue validate 70 --format json"),
            ConsoleCommand::Run("issue validate 70 --format json".to_owned())
        );
    }

    #[test]
    fn command_tokenizer_preserves_quoted_values_without_using_a_shell() {
        assert_eq!(
            tokenize_command("issue create --title \"Fix login flow\" --body-file issue.md")
                .unwrap(),
            [
                "issue",
                "create",
                "--title",
                "Fix login flow",
                "--body-file",
                "issue.md"
            ]
        );
    }

    #[test]
    fn command_palette_uses_the_existing_status_handler() {
        let directory =
            std::env::temp_dir().join(format!("conveyor-tui-dispatch-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();

        assert_eq!(
            super::execute_existing_command("status", &directory),
            "status: no active local sessions"
        );

        std::fs::remove_dir_all(directory).unwrap();
    }

    #[test]
    fn audit_rows_are_compact_human_readable_summaries() {
        assert_eq!(
            super::summarize_audit_event(
                r#"{"action":"open_draft_pr","target":"https://example.test/pr/70","actor":"sam","timestamp_unix_seconds":5}"#
            ),
            "open_draft_pr: https://example.test/pr/70"
        );
    }
}
