use conveyor_core::agent_adapter::{CommandTemplate, TemplateContext};

#[test]
fn expands_only_supported_packet_and_repository_placeholders() {
    let template = CommandTemplate {
        program: "./scripts/implement".to_owned(),
        args: vec![
            "--packet".to_owned(),
            "{packet_path}".to_owned(),
            "--issue={issue_number}".to_owned(),
        ],
    };

    let invocation = template
        .expand(&TemplateContext {
            packet_path: ".conveyor/packets/70-implementer.md".to_owned(),
            repository_root: "/work/conveyor".to_owned(),
            issue_number: 70,
            branch: "feat/70-build".to_owned(),
            role: "implementer".to_owned(),
        })
        .unwrap();

    assert_eq!(invocation.program, "./scripts/implement");
    assert_eq!(
        invocation.args,
        vec![
            "--packet",
            ".conveyor/packets/70-implementer.md",
            "--issue=70"
        ]
    );
}

#[test]
fn rejects_unknown_placeholders_before_starting_a_process() {
    let template = CommandTemplate {
        program: "agent".to_owned(),
        args: vec!["{secret}".to_owned()],
    };

    assert!(
        template
            .expand(&TemplateContext {
                packet_path: "packet.md".to_owned(),
                repository_root: "/work".to_owned(),
                issue_number: 70,
                branch: "feat/70".to_owned(),
                role: "implementer".to_owned(),
            })
            .is_err()
    );
}
