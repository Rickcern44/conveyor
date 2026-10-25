use conveyor_core::config::{Check, Config, ConfirmationAction};

#[test]
fn default_configuration_has_safe_delivery_policy() {
    let config = Config::new("octo-org/conveyor", "main");

    assert!(config.workflow.draft_prs_only);
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
fn configuration_round_trips_as_toml() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.checks.push(Check {
        name: "unit tests".to_owned(),
        command: "cargo test --workspace".to_owned(),
        required: true,
    });
    let serialized = config.to_toml().unwrap();

    assert!(serialized.contains("repository = \"octo-org/conveyor\""));
    assert!(serialized.contains("[[checks]]"));
    assert!(serialized.contains("command = \"cargo test --workspace\""));
    assert_eq!(Config::from_toml(&serialized).unwrap(), config);
}
