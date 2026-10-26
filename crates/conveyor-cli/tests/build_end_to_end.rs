use std::fs;
use std::process::Command;

use conveyor_core::config::{AgentCommand, Check, Config};

fn command(program: &str, arguments: &[&str], directory: &std::path::Path) {
    let output = Command::new(program)
        .args(arguments)
        .current_dir(directory)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{program} failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

fn temporary_directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!(
        "conveyor-e2e-build-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    fs::create_dir_all(&path).unwrap();
    path
}

#[test]
fn build_creates_a_branch_session_packet_and_local_agent_invocation() {
    let root = temporary_directory();
    let remote = root.join("remote.git");
    let repository = root.join("repository");
    let binaries = root.join("bin");
    fs::create_dir_all(&repository).unwrap();
    fs::create_dir_all(&binaries).unwrap();
    command("git", &["init", "--bare", remote.to_str().unwrap()], &root);
    command("git", &["init", "-b", "main"], &repository);
    command(
        "git",
        &["config", "user.email", "test@example.com"],
        &repository,
    );
    command("git", &["config", "user.name", "Test User"], &repository);
    fs::write(repository.join("README.md"), "fixture\n").unwrap();
    command("git", &["add", "README.md"], &repository);
    command("git", &["commit", "-m", "test fixture"], &repository);
    command(
        "git",
        &["remote", "add", "origin", remote.to_str().unwrap()],
        &repository,
    );
    command("git", &["push", "-u", "origin", "main"], &repository);

    let gh = binaries.join("gh");
    fs::write(
        &gh,
        r#"#!/bin/sh
if [ "$1" = "issue" ] && [ "$2" = "view" ]; then
  case "$*" in
    *labels*) printf 'ready-for-build\n' ;;
    *) cat <<'ISSUE'
## Problem
Build safely.
## In scope
Create a local build session.
## Out of scope
Opening a PR.
## Acceptance criteria
- Creates the branch.
## Affected components
conveyor-cli
## Verification plan
cargo test --workspace
## Dependencies and risks
None.
## Open questions
None.
ISSUE
    ;;
  esac
  exit 0
fi
if [ "$1" = "issue" ] && [ "$2" = "edit" ]; then
  printf '%s\n' "$*" >> "$CONVEYOR_GH_LOG"
  exit 0
fi
if [ "$1" = "pr" ] && [ "$2" = "create" ]; then
  printf '%s\n' "$*" >> "$CONVEYOR_GH_LOG"
  printf 'https://example.test/owner/repository/pull/70\n'
  exit 0
fi
exit 1
"#,
    )
    .unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(&gh, fs::Permissions::from_mode(0o755)).unwrap();
    }

    let mut configuration = Config::new("owner/repository", "main");
    configuration.agents.implementer = "fixture-agent".to_owned();
    configuration.agents.reviewer = "fixture-reviewer".to_owned();
    configuration.agent_commands.push(AgentCommand {
        name: "fixture-agent".to_owned(),
        program: "sh".to_owned(),
        args: vec!["-c".to_owned(), "test -f {packet_path}".to_owned()],
    });
    configuration.checks.push(Check {
        name: "intentional failure".to_owned(),
        command: "false".to_owned(),
        required: true,
    });
    configuration.agent_commands.push(AgentCommand {
        name: "fixture-reviewer".to_owned(),
        program: "sh".to_owned(),
        args: vec!["-c".to_owned(), "test -f {packet_path}".to_owned()],
    });
    fs::write(
        repository.join(".conveyor.toml"),
        configuration.to_toml().unwrap(),
    )
    .unwrap();
    command("git", &["add", ".conveyor.toml"], &repository);
    command("git", &["commit", "-m", "configure conveyor"], &repository);
    command("git", &["push"], &repository);

    let previous_path = std::env::var("PATH").unwrap();
    let path = format!("{}:{previous_path}", binaries.display());
    let gh_log = root.join("gh.log");
    let output = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .args(["build", "70", "--yes"])
        .current_dir(&repository)
        .env("PATH", path)
        .env("CONVEYOR_GH_LOG", &gh_log)
        .output()
        .unwrap();

    assert!(
        output.status.success(),
        "build failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(
            &Command::new("git")
                .args(["branch", "--show-current"])
                .current_dir(&repository)
                .output()
                .unwrap()
                .stdout,
        )
        .trim(),
        "feat/70-build"
    );
    assert!(repository.join(".conveyor/sessions/70.json").is_file());
    let session = fs::read_to_string(repository.join(".conveyor/sessions/70.json")).unwrap();
    assert!(session.contains("\"agent_command\": \"sh -c test -f"));
    assert!(session.contains("\"agent_exit_code\": 0"));
    assert!(
        repository
            .join(".conveyor/packets/70-implementer.md")
            .is_file()
    );
    assert!(
        fs::read_to_string(&gh_log)
            .unwrap()
            .contains("--add-label in-progress")
    );

    let verify = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .args(["verify"])
        .current_dir(&repository)
        .env("PATH", format!("{}:{previous_path}", binaries.display()))
        .output()
        .unwrap();
    assert!(verify.status.success());
    let review = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .args(["review", "70"])
        .current_dir(&repository)
        .env("PATH", format!("{}:{previous_path}", binaries.display()))
        .output()
        .unwrap();
    assert!(
        review.status.success(),
        "review failed: {}",
        String::from_utf8_lossy(&review.stderr)
    );
    assert!(String::from_utf8_lossy(&review.stdout).contains("reviewer completed"));
    let pr = Command::new(env!("CARGO_BIN_EXE_conveyor"))
        .args(["pr", "open", "70", "--waive-by", "rick", "--yes"])
        .current_dir(&repository)
        .env("PATH", format!("{}:{previous_path}", binaries.display()))
        .env("CONVEYOR_GH_LOG", &gh_log)
        .output()
        .unwrap();
    assert!(
        pr.status.success(),
        "pr open failed: {}",
        String::from_utf8_lossy(&pr.stderr)
    );
    assert!(repository.join(".conveyor/reviews/70.md").is_file());
    assert!(
        fs::read_to_string(repository.join(".conveyor/reviews/70-delivery.md"))
            .unwrap()
            .contains("Verification waiver: rick")
    );
    assert!(
        fs::read_to_string(repository.join(".conveyor/reports/latest.json"))
            .unwrap()
            .contains("\"waiver_by\":\"rick\"")
    );
    assert!(fs::read_to_string(&gh_log).unwrap().contains("pr create"));
    let audit = fs::read_to_string(repository.join(".conveyor/audit/70.jsonl")).unwrap();
    for action in [
        "create_branch",
        "transition_issue",
        "run_implementer",
        "run_reviewer",
        "waive_verification",
        "push_branch",
        "open_draft_pr",
    ] {
        assert!(audit.contains(&format!("\"action\":\"{action}\"")));
    }
    fs::remove_dir_all(root).unwrap();
}
