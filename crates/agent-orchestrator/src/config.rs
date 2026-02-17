use std::path::Path;

use anyhow::Context;
use serde::Deserialize;
type Result<T> = anyhow::Result<T>;

/// 编排器整体配置。
#[derive(Debug, Deserialize)]
pub struct OrchestratorConfig {
    /// 可用 Agent 列表。
    pub agents: Vec<AgentConfig>,
    /// 事件广播缓冲区大小。
    #[serde(default = "default_event_buffer_size")]
    pub event_buffer_size: usize,
}

impl OrchestratorConfig {
    /// 从 TOML 配置文件加载编排器配置。
    pub fn from_file(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("failed to read config file: {}", path.display()))?;
        Self::from_str(&content)
            .with_context(|| format!("failed to parse config file: {}", path.display()))
    }

    /// 从 TOML 字符串解析编排器配置。
    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Result<Self> {
        toml::from_str(s).context("failed to deserialize orchestrator config")
    }
}

/// 单个 Agent 的执行配置。
#[derive(Debug, Deserialize, Clone)]
pub struct AgentConfig {
    /// Agent 唯一标识。
    pub id: String,
    /// 对外展示名称。
    pub display_name: String,
    /// Agent 类型。
    #[serde(rename = "type")]
    pub agent_type: AgentType,
    /// 可执行命令。
    pub command: String,
    /// 命令参数列表。
    #[serde(default)]
    pub args: Vec<String>,
    /// 启动时注入的环境变量。
    #[serde(default)]
    pub env: Vec<EnvVar>,
    /// 是否启用该 Agent。
    #[serde(default = "default_enabled")]
    pub enabled: bool,
}

/// 环境变量键值对配置。
#[derive(Debug, Deserialize, Clone)]
pub struct EnvVar {
    /// 环境变量名。
    pub key: String,
    /// 环境变量值。
    pub value: String,
}

/// 支持的 Agent 类型。
#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AgentType {
    /// ACP（Agent Communication Protocol）类型 Agent。
    Acp,
    /// Codex 类型 Agent。
    Codex,
    /// OpenCode 类型 Agent。
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
