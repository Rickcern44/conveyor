use conveyor_core::config::{AgentCommand, Check, Config, ConfirmationAction};

#[test]
fn default_configuration_has_safe_delivery_policy() {
    let config = Config::new("octo-org/conveyor", "main");

    assert!(config.workflow.draft_prs_only);
    assert_eq!(config.workflow.in_progress_label, "in-progress");
    assert_eq!(config.conveyor.branch_pattern, "feat/{issue}-build");
    assert_eq!(
        config.workflow.require_confirmation_for,
        vec![
            ConfirmationAction::Push,
            ConfirmationAction::OpenPr,
            ConfirmationAction::Merge,
            ConfirmationAction::Deploy,
        ]
    );
}

#[test]
fn expands_the_configured_branch_pattern_for_an_issue() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.branch_pattern = "work/{issue}/implementation".to_owned();

    assert_eq!(
        config.branch_for_issue(70).unwrap(),
        "work/70/implementation"
    );
}

#[test]
fn rejects_a_branch_pattern_without_an_issue_placeholder() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.branch_pattern = "work/implementation".to_owned();

    assert!(config.to_toml().is_err());
}

#[test]
fn configuration_round_trips_as_toml() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.checks.push(Check {
        name: "unit tests".to_owned(),
        command: "cargo test --workspace".to_owned(),
        required: true,
    });
    config.agent_commands.push(AgentCommand {
        name: "local-implementer".to_owned(),
        program: "./scripts/implement".to_owned(),
        args: vec!["--packet".to_owned(), "{packet_path}".to_owned()],
    });
    let serialized = config.to_toml().unwrap();

    assert!(serialized.contains("repository = \"octo-org/conveyor\""));
    assert!(serialized.contains("[[checks]]"));
    assert!(serialized.contains("command = \"cargo test --workspace\""));
    assert!(serialized.contains("[[agent_commands]]"));
    assert!(serialized.contains("program = \"./scripts/implement\""));
    assert_eq!(Config::from_toml(&serialized).unwrap(), config);
}

#[test]
fn older_configuration_without_in_progress_label_uses_the_safe_default() {
    let source = Config::new("octo-org/conveyor", "main")
        .to_toml()
        .unwrap()
        .replace("in_progress_label = \"in-progress\"\n", "");

    assert_eq!(
        Config::from_toml(&source)
            .unwrap()
            .workflow
            .in_progress_label,
        "in-progress"
    );
}
