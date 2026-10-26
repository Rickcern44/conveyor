//! Policy for controlled draft pull-request creation.

use crate::verification::VerificationReport;

/// Whether a draft PR may be opened for the current evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PrDecision {
    Allowed,
    AllowedWithWaiver(String),
    Blocked,
}

/// Requires passing required checks unless a named operator records a waiver.
pub fn decide(report: &VerificationReport, waiver_by: Option<&str>) -> PrDecision {
    if report.passes_required_checks() {
        PrDecision::Allowed
    } else if let Some(operator) = waiver_by.filter(|operator| !operator.trim().is_empty()) {
        PrDecision::AllowedWithWaiver(operator.to_owned())
    } else {
        PrDecision::Blocked
    }
}
