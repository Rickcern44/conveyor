use conveyor_core::config::{AgentCommand, Config};
use conveyor_core::doctor::{Doctor, FindingKind, Probe, ProbeResult};

struct UnhealthyMachine;

impl Probe for UnhealthyMachine {
    fn git_repository(&self) -> ProbeResult {
        ProbeResult::Missing("not inside a Git worktree".to_owned())
    }

    fn git_remote(&self, name: &str) -> ProbeResult {
        ProbeResult::Missing(format!("Git remote `{name}` is not configured"))
    }

    fn github_authentication(&self) -> ProbeResult {
        ProbeResult::Missing("gh auth status did not find an active account".to_owned())
    }

    fn executable(&self, name: &str) -> ProbeResult {
        ProbeResult::Missing(format!("{name} not found on PATH"))
    }
}

#[test]
fn reports_every_missing_prerequisite_without_stopping_early() {
    let report = Doctor::new(&UnhealthyMachine).check(&Config::new("octo-org/conveyor", "main"));

    assert!(!report.is_healthy());
    assert_eq!(report.findings.len(), 6);
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::GitRepository)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::GithubAuthentication)
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.kind == FindingKind::GitRemote)
    );
    assert!(
        report
            .findings
            .iter()
            .filter(|finding| finding.kind == FindingKind::AgentExecutable)
            .count()
            == 3
    );
}

#[test]
fn checks_the_program_in_a_configured_agent_template() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.agents.implementer = "local-implementer".to_owned();
    config.agent_commands.push(AgentCommand {
        name: "local-implementer".to_owned(),
        program: "./scripts/implement".to_owned(),
        args: Vec::new(),
    });

    let report = Doctor::new(&UnhealthyMachine).check(&config);

    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.detail.contains("./scripts/implement"))
    );
}
