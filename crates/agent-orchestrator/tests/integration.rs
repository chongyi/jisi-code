use std::path::PathBuf;
use std::time::Duration;

use agent_orchestrator::{
    Orchestrator, OrchestratorConfig, OrchestratorError, OrchestratorEvent, SessionId,
};

fn basic_config_toml() -> &'static str {
    r#"
event_buffer_size = 256

[[agents]]
id = "agent-a"
display_name = "Agent A"
type = "acp"
command = "echo"
args = ["hello"]
enabled = true

[[agents]]
id = "agent-b"
display_name = "Agent B"
type = "codex"
command = "codex"
enabled = true
"#
}

#[test]
fn test_config_parsing() {
    let config = OrchestratorConfig::from_str(basic_config_toml()).expect("config should parse");

    assert_eq!(config.event_buffer_size, 256);
    assert_eq!(config.agents.len(), 2);

    let first = &config.agents[0];
    assert_eq!(first.id, "agent-a");
    assert_eq!(first.display_name, "Agent A");
    assert!(first.enabled);
    assert_eq!(first.command, "echo");
    assert_eq!(first.args, vec!["hello"]);
}

#[test]
fn test_orchestrator_creation() {
    let config = OrchestratorConfig::from_str(basic_config_toml()).expect("config should parse");
    let orchestrator = Orchestrator::new(config).expect("orchestrator should initialize");

    let agents = orchestrator.available_agents();
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[0].id, "agent-a");
    assert_eq!(agents[1].id, "agent-b");
}

#[test]
fn test_list_agents() {
    let config = OrchestratorConfig::from_str(
        r#"
event_buffer_size = 64

[[agents]]
id = "enabled-agent"
display_name = "Enabled Agent"
type = "acp"
command = "echo"
enabled = true

[[agents]]
id = "disabled-agent"
display_name = "Disabled Agent"
type = "acp"
command = "echo"
enabled = false
"#,
    )
    .expect("config should parse");

    let orchestrator = Orchestrator::new(config).expect("orchestrator should initialize");
    let agents = orchestrator.available_agents();

    assert_eq!(agents.len(), 1);
    assert_eq!(agents[0].id, "enabled-agent");
}

#[test]
fn test_subscribe_events() {
    let config = OrchestratorConfig::from_str(basic_config_toml()).expect("config should parse");
    let orchestrator = Orchestrator::new(config).expect("orchestrator should initialize");

    let mut stream = orchestrator.subscribe_events();
    assert!(stream.try_recv().is_err());
}

#[tokio::test]
async fn test_session_not_found() {
    let config = OrchestratorConfig::from_str(basic_config_toml()).expect("config should parse");
    let orchestrator = Orchestrator::new(config).expect("orchestrator should initialize");

    let missing_session = SessionId::new();
    let result = orchestrator
        .send_prompt(&missing_session, "hello")
        .await
        .expect_err("send_prompt should fail for missing session");

    match result {
        OrchestratorError::SessionNotFound(id) => assert_eq!(id, missing_session.to_string()),
        other => panic!("expected SessionNotFound, got: {other:?}"),
    }
}

#[tokio::test]
#[ignore = "requires local Claude Code SDK process"]
async fn test_full_workflow() {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let config_path = manifest_dir.join("../../agents.toml");

    let config = OrchestratorConfig::from_file(&config_path)
        .expect("should load config from workspace agents.toml");
    let orchestrator = Orchestrator::new(config).expect("orchestrator should initialize");

    let mut events = orchestrator.subscribe_events();
    let session = orchestrator
        .create_session("claude-code-sdk", &manifest_dir)
        .await
        .expect("session should be created");

    orchestrator
        .send_prompt(session.id(), "Reply with one short line")
        .await
        .expect("prompt should be sent");

    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    let mut got_content_delta = false;

    while tokio::time::Instant::now() < deadline {
        let remaining = deadline.saturating_duration_since(tokio::time::Instant::now());

        match tokio::time::timeout(remaining.min(Duration::from_secs(3)), events.recv()).await {
            Ok(Ok(OrchestratorEvent::ContentDelta { content, .. })) => {
                if !content.trim().is_empty() {
                    got_content_delta = true;
                    break;
                }
            }
            Ok(Ok(OrchestratorEvent::SessionError { error, .. })) => {
                panic!("received SessionError during workflow: {error}");
            }
            Ok(Ok(_)) => {}
            Ok(Err(_)) => {}
            Err(_) => {}
        }
    }

    orchestrator
        .close_session(session.id())
        .await
        .expect("session should close");

    assert!(
        got_content_delta,
        "expected to receive at least one non-empty ContentDelta event"
    );
}
