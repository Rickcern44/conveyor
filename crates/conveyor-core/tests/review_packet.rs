use conveyor_core::evidence::EvidenceReport;
use conveyor_core::issue_contract::IssueContract;
use conveyor_core::review_packet::ReviewPacket;
use conveyor_core::verification::{CheckResult, Outcome, VerificationReport};

#[test]
fn renders_contract_diff_and_verification_evidence_for_review() {
    let packet = ReviewPacket {
        issue_number: 70,
        contract: IssueContract {
            problem: "Prevent unsafe builds.".to_owned(),
            in_scope: "Build preflight.".to_owned(),
            out_of_scope: "PR creation.".to_owned(),
            acceptance_criteria: "- Requires confirmation.".to_owned(),
            affected_components: "conveyor-cli".to_owned(),
            verification_plan: "cargo test --workspace".to_owned(),
            dependencies_risks: "GitHub access.".to_owned(),
            open_questions: "None.".to_owned(),
            delivery_slice: None,
        },
        diff_summary: "2 files changed, 20 insertions".to_owned(),
        evidence: EvidenceReport::new(
            "abc123",
            VerificationReport {
                results: vec![CheckResult {
                    name: "unit tests".to_owned(),
                    command: "cargo test --workspace".to_owned(),
                    required: true,
                    outcome: Outcome::Passed,
                    exit_code: Some(0),
                    duration_ms: 12,
                }],
            },
        ),
    };

    let markdown = packet.to_markdown();

    assert!(markdown.contains("# Conveyor review packet: issue #70"));
    assert!(markdown.contains("2 files changed, 20 insertions"));
    assert!(markdown.contains("revision: `abc123`"));
    assert!(markdown.contains("unit tests: passed"));
}
