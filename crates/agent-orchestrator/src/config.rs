use std::path::Path;

use anyhow::Context;
use serde::Deserialize;
type Result<T> = anyhow::Result<T>;

#[derive(Debug, Deserialize)]
pub struct OrchestratorConfig {
    pub agents: Vec<AgentConfig>,
    #[serde(default = "default_event_buffer_size")]
    pub event_buffer_size: usize,
}

impl OrchestratorConfig {
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        Self::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).context("failed to deserialize orchestrator config")
    }
}

#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    pub id: String,
    pub display_name: String,
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    pub command: String,
    #[serde(default)]
    pub args: Vec<String>,
    #[serde(default)]
    pub env: Vec<EnvVar>,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

#[derive(Debug, Deserialize, Clone)]
pub struct EnvVar {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    Acp,
    Codex,
    OpenCode,
}

fn default_event_buffer_size() -> usize {
    1_000
}

fn default_enabled() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::{AgentType, OrchestratorConfig};

    #[test]
    fn test_parse_config() {
        let raw = r#"
event_buffer_size = 2048

[[agents]]
id = "claude-acp"
display_name = "Claude Code ACP"
type = "acp"
command = "claude"
args = ["agent", "--transport", "stdio"]
enabled = true

[[agents.env]]
key = "CLAUDE_CODE_ENTRYPOINT"
value = "agent-orchestrator"

[[agents]]
id = "codex-default"
display_name = "Codex CLI"
type = "codex"
command = "codex"
"#;

        let config = OrchestratorConfig::from_str(raw).expect("config should parse");
        assert_eq!(config.event_buffer_size, 2048);
        assert_eq!(config.agents.len(), 2);

        let acp = &config.agents[0];
        assert_eq!(acp.id, "claude-acp");
        assert_eq!(acp.display_name, "Claude Code ACP");
        assert_eq!(acp.agent_type, AgentType::Acp);
        assert_eq!(acp.command, "claude");
        assert_eq!(acp.args, vec!["agent", "--transport", "stdio"]);
        assert!(acp.enabled);
        assert_eq!(acp.env.len(), 1);
        assert_eq!(acp.env[0].key, "CLAUDE_CODE_ENTRYPOINT");
        assert_eq!(acp.env[0].value, "agent-orchestrator");

        let codex = &config.agents[1];
        assert_eq!(codex.agent_type, AgentType::Codex);
        assert!(codex.args.is_empty());
        assert!(codex.env.is_empty());
        assert!(codex.enabled);
    }
}
