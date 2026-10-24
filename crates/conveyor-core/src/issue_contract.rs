//! Validation rules for the implementation contract stored in a GitHub issue.

/// The material implementation information extracted from an issue.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IssueContract {
    pub problem: String,
    pub in_scope: String,
    pub out_of_scope: String,
    pub acceptance_criteria: String,
    pub affected_components: String,
    pub verification_plan: String,
    pub dependencies_risks: String,
    pub open_questions: String,
    pub delivery_slice: Option<String>,
}

/// A field that Conveyor requires before an issue is ready for build.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Requirement {
    Problem,
    InScope,
    OutOfScope,
    AcceptanceCriteria,
    AffectedComponents,
    VerificationPlan,
    DependenciesRisks,
    OpenQuestions,
}

impl Requirement {
    pub fn label(self) -> &'static str {
        match self {
            Self::Problem => "problem statement",
            Self::InScope => "in-scope behavior",
            Self::OutOfScope => "out-of-scope behavior",
            Self::AcceptanceCriteria => "acceptance criteria",
            Self::AffectedComponents => "affected components or discovery task",
            Self::VerificationPlan => "test or verification expectations",
            Self::DependenciesRisks => "dependencies and risks",
            Self::OpenQuestions => "open questions",
        }
    }
}

/// The result of checking an issue against Conveyor's readiness requirements.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationReport {
    pub missing: Vec<Requirement>,
}

/// Extracts the standard Conveyor issue-template sections from Markdown.
///
/// Unknown headings are ignored, allowing repositories to add their own issue
/// context without changing the readiness contract.
pub fn from_markdown(markdown: &str) -> IssueContract {
    let mut contract = IssueContract {
        problem: String::new(),
        in_scope: String::new(),
        out_of_scope: String::new(),
        acceptance_criteria: String::new(),
        affected_components: String::new(),
        verification_plan: String::new(),
        dependencies_risks: String::new(),
        open_questions: String::new(),
        delivery_slice: None,
    };
    let mut heading = None;
    let mut content = String::new();
    for line in markdown.lines() {
        if let Some(next_heading) = line.strip_prefix("## ") {
            assign_section(&mut contract, heading.take(), &content);
            heading = Some(next_heading.trim().to_ascii_lowercase());
            content.clear();
        } else if heading.is_some() {
            content.push_str(line);
            content.push('\n');
        }
    }
    assign_section(&mut contract, heading, &content);
    contract
}

fn assign_section(contract: &mut IssueContract, heading: Option<String>, content: &str) {
    let value = content.trim().to_owned();
    match heading.as_deref() {
        Some("problem") => contract.problem = value,
        Some("in scope") => contract.in_scope = value,
        Some("out of scope") => contract.out_of_scope = value,
        Some("acceptance criteria") => contract.acceptance_criteria = value,
        Some("affected components") | Some("affected components or discovery task") => {
            contract.affected_components = value
        }
        Some("verification plan") | Some("test and verification expectations") => {
            contract.verification_plan = value
        }
        Some("dependencies and risks") => contract.dependencies_risks = value,
        Some("open questions") => contract.open_questions = value,
        Some("delivery slice") => contract.delivery_slice = (!value.is_empty()).then_some(value),
        _ => {}
    }
}

impl ValidationReport {
    pub fn is_valid(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Validates a complete issue contract without performing any state transition.
pub fn validate(contract: &IssueContract) -> ValidationReport {
    let fields = [
        (Requirement::Problem, contract.problem.as_str()),
        (Requirement::InScope, contract.in_scope.as_str()),
        (Requirement::OutOfScope, contract.out_of_scope.as_str()),
        (
            Requirement::AcceptanceCriteria,
            contract.acceptance_criteria.as_str(),
        ),
        (
            Requirement::AffectedComponents,
            contract.affected_components.as_str(),
        ),
        (
            Requirement::VerificationPlan,
            contract.verification_plan.as_str(),
        ),
        (
            Requirement::DependenciesRisks,
            contract.dependencies_risks.as_str(),
        ),
        (Requirement::OpenQuestions, contract.open_questions.as_str()),
    ];
    ValidationReport {
        missing: fields
            .into_iter()
            .filter_map(|(requirement, value)| value.trim().is_empty().then_some(requirement))
            .collect(),
    }
}
