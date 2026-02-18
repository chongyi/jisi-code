use crate::SessionModelConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;

/// 客户端发送的 WebSocket 消息。
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ClientMessage {
    /// 请求创建新会话。
    CreateSession {
        agent_id: String,
        project_path: String,
        #[serde(default)]
        model_config: Option<SessionModelConfig>,
    },
    /// 向指定会话发送提示词。
    SendPrompt { session_id: String, prompt: String },
    /// 请求关闭指定会话。
    CloseSession { session_id: String },
    /// 查询可用 Agent 列表。
    ListAgents,
    /// 查询活跃会话列表。
    ListSessions,
}

/// 服务端发送的 WebSocket 消息。
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ServerMessage {
    /// 会话创建成功。
    SessionCreated {
        session_id: String,
        agent_name: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        model_config: Option<SessionModelConfig>,
    },
    /// 模型增量输出。
    ContentDelta { session_id: String, content: String },
    /// 工具调用通知。
    ToolCall {
        session_id: String,
        tool_name: String,
        args: Value,
    },
    /// 文件变更通知。
    FileChange {
        session_id: String,
        path: String,
        action: String,
        content: Option<String>,
        diff: Option<String>,
    },
    /// Token 使用信息。
    TokenUsage { session_id: String, usage: Value },
    /// 思考/推理过程。
    Thinking { session_id: String, content: String },
    /// 会话关闭通知。
    SessionClosed { session_id: String },
    /// 提示词已接收。
    PromptAccepted { session_id: String },
    /// Agent 列表响应。
    AgentList { agents: Vec<AgentInfoMessage> },
    /// 会话列表响应。
    SessionList { sessions: Vec<SessionInfoMessage> },
    /// 错误消息。
    Error { message: String },
}

/// Agent 信息（WebSocket 传输用）。
#[derive(Debug, Serialize, Deserialize)]
pub struct AgentInfoMessage {
    pub id: String,
    pub display_name: String,
    pub agent_type: String,
    pub enabled: bool,
}

/// 会话信息（WebSocket 传输用）。
#[derive(Debug, Serialize, Deserialize)]
pub struct SessionInfoMessage {
    pub session_id: String,
    pub agent_name: String,
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_config: Option<SessionModelConfig>,
}
