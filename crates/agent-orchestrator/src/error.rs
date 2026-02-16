use thiserror::Error;

#[derive(Debug, Error)]
pub enum OrchestratorError {
    #[error("配置错误: {0}")]
    Config(String),

    #[error("执行器错误: {0}")]
    Executor(String),

    #[error("会话未找到: {0}")]
    SessionNotFound(String),

    #[error("Agent 未找到: {0}")]
    AgentNotFound(String),

    #[error("不支持的 Agent 类型")]
    UnsupportedAgentType,

    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML 错误: {0}")]
    Toml(#[from] toml::de::Error),

    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

pub type Result<T> = std::result::Result<T, OrchestratorError>;
