//! Local configuration and environment-preflight commands.

use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use conveyor_core::config::Config;
use conveyor_core::doctor::{Doctor, FindingKind, Probe, ProbeResult};

pub(crate) fn initialize(
    repository: &str,
    target_branch: &str,
    directory: &Path,
) -> Result<String, String> {
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

pub(crate) fn doctor(directory: &Path) -> Result<String, String> {
    let configuration = crate::commands::workflow::load_config(directory)?;
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

#[cfg(test)]
mod tests {
    use super::initialize;

    #[test]
    fn initialize_creates_the_requested_configuration() {
        let directory =
            std::env::temp_dir().join(format!("conveyor-setup-test-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&directory);
        std::fs::create_dir_all(&directory).unwrap();

        initialize("owner/repository", "trunk", &directory).unwrap();

        let config = std::fs::read_to_string(directory.join(".conveyor.toml")).unwrap();
        assert!(config.contains("repository = \"owner/repository\""));
        assert!(config.contains("target_branch = \"trunk\""));
        std::fs::remove_dir_all(directory).unwrap();
    }
}
