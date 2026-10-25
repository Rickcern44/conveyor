use conveyor_core::config::Check;
use conveyor_core::verification::{CommandResult, CommandRunner, Outcome, verify};

struct FakeRunner;

impl CommandRunner for FakeRunner {
    fn run(&self, command: &str) -> CommandResult {
        match command {
            "passes" => CommandResult::completed(0, 12),
            _ => CommandResult::completed(1, 8),
        }
    }
}

#[test]
fn records_all_configured_checks_and_fails_when_a_required_check_fails() {
    let checks = vec![
        Check {
            name: "unit tests".to_owned(),
            command: "passes".to_owned(),
            required: true,
        },
        Check {
            name: "optional report".to_owned(),
            command: "fails".to_owned(),
            required: false,
        },
    ];

    let report = verify(&FakeRunner, &checks);

    assert!(report.passes_required_checks());
    assert_eq!(report.results[0].outcome, Outcome::Passed);
    assert_eq!(report.results[1].outcome, Outcome::Failed);
    assert_eq!(report.results[1].duration_ms, 8);
}
