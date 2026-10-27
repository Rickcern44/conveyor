use conveyor_core::config::{AcpAgentCommand, AgentCommand, Check, Config, ConfirmationAction};

#[test]
fn default_configuration_has_safe_delivery_policy() {
    let config = Config::new("octo-org/conveyor", "main");

    assert!(config.workflow.draft_prs_only);
    assert_eq!(config.workflow.ready_label, "conveyor:ready");
    assert_eq!(config.workflow.in_progress_label, "conveyor:in-progress");
    assert_eq!(config.workflow.blocked_label, "conveyor:blocked");
    assert_eq!(config.conveyor.branch_pattern, "feat/{issue}-build");
    assert_eq!(config.conveyor.remote, "origin");
    assert_eq!(
        config.conveyor.verification_report_path,
        ".conveyor/reports/latest.json"
    );
    assert_eq!(
        config.conveyor.commit_message_pattern,
        "feat: issue #{issue}"
    );
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
fn configuration_round_trips_a_custom_delivery_remote() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.remote = "fork".to_owned();

    assert_eq!(
        Config::from_toml(&config.to_toml().unwrap()).unwrap(),
        config
    );
}

#[test]
fn configuration_round_trips_an_optional_pr_template_path() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.pr_template_path = Some(".github/pull_request_template.md".to_owned());

    let serialized = config.to_toml().unwrap();

    assert!(serialized.contains("pr_template_path = \".github/pull_request_template.md\""));
    assert_eq!(Config::from_toml(&serialized).unwrap(), config);
}

#[test]
fn configuration_round_trips_a_custom_verification_report_path() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.verification_report_path = ".conveyor/evidence/current.json".to_owned();

    assert_eq!(
        Config::from_toml(&config.to_toml().unwrap()).unwrap(),
        config
    );
}

#[test]
fn configuration_round_trips_an_acp_agent_process() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.acp_agents.push(AcpAgentCommand {
        name: "codex-acp".to_owned(),
        program: "npx".to_owned(),
        args: vec![
            "-y".to_owned(),
            "@agentclientprotocol/codex-acp@latest".to_owned(),
        ],
    });

    assert_eq!(
        Config::from_toml(&config.to_toml().unwrap()).unwrap(),
        config
    );
    assert_eq!(config.acp_agent("codex-acp").unwrap().program, "npx");
}

#[test]
fn built_in_acp_presets_require_only_the_provider_name() {
    assert_eq!(
        AcpAgentCommand::preset("codex").unwrap().args,
        vec!["-y", "@agentclientprotocol/codex-acp"]
    );
    assert_eq!(
        AcpAgentCommand::preset("claude").unwrap().args,
        vec!["-y", "@agentclientprotocol/claude-agent-acp"]
    );
}

#[test]
fn default_roles_resolve_to_acp_provider_presets() {
    let config = Config::new("octo-org/conveyor", "main");

    assert_eq!(config.agent_program("codex").as_deref(), Some("npx"));
    assert_eq!(config.agent_program("claude").as_deref(), Some("npx"));
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
fn expands_the_configured_commit_message_pattern_for_an_issue() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.commit_message_pattern = "feat(auth): issue #{issue}".to_owned();

    assert_eq!(
        config.commit_message_for_issue(70).unwrap(),
        "feat(auth): issue #70"
    );
}

#[test]
fn rejects_a_branch_pattern_without_an_issue_placeholder() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.conveyor.branch_pattern = "work/implementation".to_owned();

    assert!(config.to_toml().is_err());
}

#[test]
fn confirmation_policy_can_be_queried_for_each_delivery_action() {
    let mut config = Config::new("octo-org/conveyor", "main");
    config.workflow.require_confirmation_for = vec![ConfirmationAction::OpenPr];

    assert!(!config.requires_confirmation(ConfirmationAction::Push));
    assert!(config.requires_confirmation(ConfirmationAction::OpenPr));
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
        .replace("in_progress_label = \"conveyor:in-progress\"\n", "");

    assert_eq!(
        Config::from_toml(&source)
            .unwrap()
            .workflow
            .in_progress_label,
        "conveyor:in-progress"
    );
}
