use conveyor_core::pr_gate::{PrDecision, decide, delivery_summary};
use conveyor_core::verification::{CheckResult, Outcome, VerificationReport};

fn report(outcome: Outcome) -> VerificationReport {
    VerificationReport {
        results: vec![CheckResult {
            name: "unit tests".to_owned(),
            command: "cargo test --workspace".to_owned(),
            required: true,
            outcome,
            exit_code: Some(if outcome == Outcome::Passed { 0 } else { 1 }),
            duration_ms: 1,
        }],
    }
}

#[test]
fn allows_draft_pr_only_when_required_checks_pass() {
    assert_eq!(decide(&report(Outcome::Passed), None), PrDecision::Allowed);
}

#[test]
fn requires_a_named_waiver_when_required_checks_fail() {
    assert_eq!(decide(&report(Outcome::Failed), None), PrDecision::Blocked);
    assert_eq!(
        decide(&report(Outcome::Failed), Some("rick")),
        PrDecision::AllowedWithWaiver("rick".to_owned())
    );
}

#[test]
fn delivery_summary_makes_a_waiver_auditable_in_the_pr_body() {
    let summary = delivery_summary("# Review packet", Some("rick"));

    assert!(summary.contains("# Review packet"));
    assert!(summary.contains("Verification waiver: rick"));
}
