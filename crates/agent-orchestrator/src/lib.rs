//! Agent Orchestrator - AI 编码工具统一编排模块。
//!
//! 该 crate 提供统一的 API 来编排和管理多种 AI 编码助手（如 Claude Code）。
//! 核心组件包括 [`Orchestrator`] 统一入口、[`SessionManager`] 会话管理、
//! [`Executor`] 执行器抽象和 [`EventBroadcaster`] 事件系统。

/// 配置模型与解析能力。
pub mod config;
/// 错误类型与统一结果别名。
pub mod error;
/// 事件定义与广播/订阅能力。
pub mod events;
/// 执行器抽象与具体实现。
pub mod executor;
/// 编排器统一入口。
pub mod orchestrator;
/// 会话模型与会话管理。
pub mod session;
/// WebSocket API 模块（需启用 `ws-api` feature）。
#[cfg(feature = "ws-api")]
pub mod ws_api;

pub use config::{AgentConfig, AgentType, EnvVar, OrchestratorConfig};
pub use error::{OrchestratorError, Result};
pub use events::{EventBroadcaster, EventStream, OrchestratorEvent};
pub use executor::{
    AcpExecutor, ClaudeSdkExecutor, CodexExecutor, CodexModelOptions, Executor, OpenCodeExecutor,
    OpenCodeModelOptions, ReasoningEffort,
};
pub use orchestrator::{AgentInfo, Orchestrator};
pub use session::{
    Session, SessionId, SessionManager, SessionModelConfig, SessionReasoningEffort, SessionStatus,
};
