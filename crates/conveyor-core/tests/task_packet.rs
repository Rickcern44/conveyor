use conveyor_core::config::Check;
use conveyor_core::issue_contract::IssueContract;
use conveyor_core::task_packet::{AgentRole, TaskPacket};

fn contract() -> IssueContract {
    IssueContract {
        problem: "Users need reliable issue handoffs.".to_owned(),
        in_scope: "Generate a task packet.".to_owned(),
        out_of_scope: "Starting an agent.".to_owned(),
        acceptance_criteria: "- Includes required checks.".to_owned(),
        affected_components: "conveyor-core".to_owned(),
        verification_plan: "cargo test --workspace".to_owned(),
        dependencies_risks: "None.".to_owned(),
        open_questions: "None.".to_owned(),
        delivery_slice: None,
    }
}

#[test]
fn renders_a_portable_implementer_brief() {
    let packet = TaskPacket::new(
        70,
        AgentRole::Implementer,
        "feat/70-task-packet",
        contract(),
        "Follow AGENTS.md and use TDD.",
        vec![Check {
            name: "unit tests".to_owned(),
            command: "cargo test --workspace".to_owned(),
            required: true,
        }],
    );

    let markdown = packet.to_markdown();

    assert!(markdown.contains("# Conveyor task packet: issue #70"));
    assert!(markdown.contains("Role: implementer"));
    assert!(markdown.contains("Branch: `feat/70-task-packet`"));
    assert!(markdown.contains("Users need reliable issue handoffs."));
    assert!(markdown.contains("`cargo test --workspace`"));
    assert!(markdown.contains("Follow AGENTS.md and use TDD."));
}

#[test]
fn accepts_only_known_agent_roles() {
    assert_eq!(AgentRole::parse("planner"), Some(AgentRole::Planner));
    assert_eq!(
        AgentRole::parse("implementer"),
        Some(AgentRole::Implementer)
    );
    assert_eq!(AgentRole::parse("reviewer"), Some(AgentRole::Reviewer));
    assert_eq!(AgentRole::parse("publisher"), None);
}
