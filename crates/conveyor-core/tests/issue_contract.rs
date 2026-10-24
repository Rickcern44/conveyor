use conveyor_core::issue_contract::{IssueContract, Requirement, from_markdown, validate};

fn complete_contract() -> IssueContract {
    IssueContract {
        problem: "Operators need a repeatable handoff.".to_owned(),
        in_scope: "Validate prepared GitHub issues.".to_owned(),
        out_of_scope: "Creating branches.".to_owned(),
        acceptance_criteria: "- Validation reports missing requirements.".to_owned(),
        affected_components: "conveyor-core and conveyor-cli".to_owned(),
        verification_plan: "cargo test --workspace".to_owned(),
        dependencies_risks: "GitHub authentication is required.".to_owned(),
        open_questions: "None.".to_owned(),
        delivery_slice: None,
    }
}

#[test]
fn complete_contract_passes_validation() {
    let report = validate(&complete_contract());

    assert!(report.is_valid());
    assert!(report.missing.is_empty());
}

#[test]
fn missing_material_fields_are_reported_together() {
    let contract = IssueContract {
        problem: String::new(),
        verification_plan: "  ".to_owned(),
        ..complete_contract()
    };

    let report = validate(&contract);

    assert!(!report.is_valid());
    assert_eq!(
        report.missing,
        vec![Requirement::Problem, Requirement::VerificationPlan]
    );
}

#[test]
fn extracts_contract_sections_from_a_github_issue_body() {
    let contract = from_markdown(
        r#"
## Problem
Operators need an auditable workflow.

## In scope
Validate readiness.

## Out of scope
Creating pull requests.

## Acceptance criteria
- Shows every missing field.

## Affected components
conveyor-core

## Verification plan
cargo test --workspace

## Dependencies and risks
GitHub access is required.

## Open questions
None.
"#,
    );

    assert!(validate(&contract).is_valid());
    assert_eq!(contract.problem, "Operators need an auditable workflow.");
    assert_eq!(contract.affected_components, "conveyor-core");
}
