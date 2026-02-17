use thiserror::Error;

/// 编排器统一错误类型。
#[derive(Debug, Error)]
pub enum OrchestratorError {
    /// 配置读取或解析失败。
    #[error("配置错误: {0}")]
    Config(String),

    /// 执行器生命周期或协议交互失败。
    #[error("执行器错误: {0}")]
    Executor(String),

    /// 指定会话不存在。
    #[error("会话未找到: {0}")]
    SessionNotFound(String),

    /// 指定 Agent 不存在或不可用。
    #[error("Agent 未找到: {0}")]
    AgentNotFound(String),

    /// 当前 Agent 类型尚未实现对应执行器。
    #[error("不支持的 Agent 类型")]
    UnsupportedAgentType,

    /// IO 层错误。
    #[error("IO 错误: {0}")]
    Io(#[from] std::io::Error),

    /// JSON 序列化/反序列化错误。
    #[error("JSON 错误: {0}")]
    Json(#[from] serde_json::Error),

    /// TOML 反序列化错误。
    #[error("TOML 错误: {0}")]
    Toml(#[from] toml::de::Error),

    /// 其他来源的通用错误。
    #[error("其他错误: {0}")]
    Other(#[from] anyhow::Error),
}

/// 编排器统一 `Result` 别名。
pub type Result<T> = std::result::Result<T, OrchestratorError>;
