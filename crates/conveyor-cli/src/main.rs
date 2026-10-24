use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::Command;

use conveyor_core::config::Config;
use conveyor_core::doctor::{Doctor, FindingKind, Probe, ProbeResult};

fn main() {
    let arguments = std::env::args().skip(1);
    match run(arguments, &current_directory()) {
        Ok(message) => println!("{message}"),
        Err(error) => {
            eprintln!("conveyor: {error}");
            std::process::exit(1);
        }
    }
}

fn current_directory() -> std::path::PathBuf {
    std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."))
}

fn run<I, S>(arguments: I, directory: &Path) -> Result<String, String>
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
        "--help" | "-h" | "help" => Ok(usage().to_owned()),
        _ => Err(format!("unknown command: {command}\n\n{}", usage())),
    }
}

fn doctor(directory: &Path) -> Result<String, String> {
    let path = directory.join(".conveyor.toml");
    let source = std::fs::read_to_string(&path)
        .map_err(|error| format!("could not read {}: {error}", path.display()))?;
    let configuration = Config::from_toml(&source)
        .map_err(|error| format!("could not parse {}: {error}", path.display()))?;
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
        FindingKind::GithubAuthentication => "github authentication",
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

    fn github_authentication(&self) -> ProbeResult {
        command_succeeds("gh", ["auth", "status"], Some(self.directory)).map_or_else(
            || ProbeResult::Missing("run `gh auth login` to authenticate".to_owned()),
            |_| ProbeResult::Available,
        )
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
    "Usage:\n  conveyor init --repository <owner/repository> [--target-branch <branch>]\n  conveyor doctor"
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::run;

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
    fn init_never_overwrites_an_existing_configuration() {
        let directory = temporary_directory("init-no-overwrite");
        let path = directory.join(".conveyor.toml");
        fs::write(&path, "preserve this file").unwrap();

        let error = run(["init", "--repository", "octo-org/conveyor"], &directory).unwrap_err();

        assert!(error.contains("already exists"));
        assert_eq!(fs::read_to_string(&path).unwrap(), "preserve this file");
        fs::remove_dir_all(directory).unwrap();
    }
}
