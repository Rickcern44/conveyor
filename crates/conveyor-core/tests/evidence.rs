use conveyor_core::evidence::EvidenceReport;
use conveyor_core::verification::{CheckResult, Outcome, VerificationReport};

#[test]
fn serializes_revision_and_complete_check_evidence_as_json() {
    let evidence = EvidenceReport::new(
        "abc123",
        VerificationReport {
            results: vec![CheckResult {
                name: "unit tests".to_owned(),
                command: "cargo test --workspace".to_owned(),
                required: true,
                outcome: Outcome::Passed,
                exit_code: Some(0),
                duration_ms: 42,
            }],
        },
    );

    let json = evidence.to_json();

    assert!(json.contains("\"revision\":\"abc123\""));
    assert!(json.contains("\"command\":\"cargo test --workspace\""));
    assert!(json.contains("\"duration_ms\":42"));
    assert!(json.contains("\"passed_required_checks\":true"));
}
