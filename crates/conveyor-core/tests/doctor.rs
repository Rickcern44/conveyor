use conveyor_core::config::Config;
use conveyor_core::doctor::{Doctor, FindingKind, Probe, ProbeResult};

struct UnhealthyMachine;

impl Probe for UnhealthyMachine {
    fn git_repository(&self) -> ProbeResult {
        ProbeResult::Missing("not inside a Git worktree".to_owned())
    }

    fn github_authentication(&self) -> ProbeResult {
        ProbeResult::Missing("gh auth status did not find an active account".to_owned())
    }

    fn executable(&self, _name: &str) -> ProbeResult {
        ProbeResult::Missing("not found on PATH".to_owned())
    }
}

#[test]
fn reports_every_missing_prerequisite_without_stopping_early() {
    let report = Doctor::new(&UnhealthyMachine).check(&Config::new("octo-org/conveyor", "main"));

    assert!(!report.is_healthy());
    assert_eq!(report.findings.len(), 5);
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
            .filter(|finding| finding.kind == FindingKind::AgentExecutable)
            .count()
            == 3
    );
}
