//! Read-only environment diagnostic rules.

use crate::config::Config;

/// A read-only observation of one local prerequisite.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProbeResult {
    Available,
    Missing(String),
}

/// Queries machine-specific prerequisites without allowing any mutations.
pub trait Probe {
    fn git_repository(&self) -> ProbeResult;
    fn github_authentication(&self) -> ProbeResult;
    fn executable(&self, name: &str) -> ProbeResult;
}

/// Runs the standard Conveyor environment checks.
pub struct Doctor<'a, P> {
    probe: &'a P,
}

impl<'a, P: Probe> Doctor<'a, P> {
    pub fn new(probe: &'a P) -> Self {
        Self { probe }
    }

    /// Produces all findings, including independent failures, in a single pass.
    pub fn check(&self, configuration: &Config) -> DoctorReport {
        let mut findings = Vec::new();
        collect(
            &mut findings,
            FindingKind::GitRepository,
            self.probe.git_repository(),
        );
        collect(
            &mut findings,
            FindingKind::GithubAuthentication,
            self.probe.github_authentication(),
        );
        for executable in [
            configuration.agents.planner.as_str(),
            configuration.agents.implementer.as_str(),
            configuration.agents.reviewer.as_str(),
        ] {
            collect(
                &mut findings,
                FindingKind::AgentExecutable,
                self.probe.executable(executable),
            );
        }
        DoctorReport { findings }
    }
}

fn collect(findings: &mut Vec<Finding>, kind: FindingKind, result: ProbeResult) {
    if let ProbeResult::Missing(detail) = result {
        findings.push(Finding { kind, detail });
    }
}

/// The aggregate result of a doctor run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DoctorReport {
    pub findings: Vec<Finding>,
}

impl DoctorReport {
    pub fn is_healthy(&self) -> bool {
        self.findings.is_empty()
    }
}

/// A prerequisite that needs operator attention.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Finding {
    pub kind: FindingKind,
    pub detail: String,
}

/// A category of doctor finding.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FindingKind {
    GitRepository,
    GithubAuthentication,
    AgentExecutable,
}
