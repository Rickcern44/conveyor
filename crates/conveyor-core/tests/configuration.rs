use conveyor_core::config::{Config, ConfirmationAction};

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
    let config = Config::new("octo-org/conveyor", "main");
    let serialized = config.to_toml().unwrap();

    assert!(serialized.contains("repository = \"octo-org/conveyor\""));
    assert_eq!(Config::from_toml(&serialized).unwrap(), config);
}
