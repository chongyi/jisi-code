//! 会话模型与会话管理模块。

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use uuid::Uuid;

/// 会话管理器实现。
pub mod manager;
/// 导出会话管理器类型。
pub use manager::SessionManager;

/// 会话唯一标识。
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(Uuid);

impl SessionId {
    /// 生成新的随机会话 ID。
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    /// 从字符串解析会话 ID。
    pub fn from_string(value: &str) -> Result<Self, uuid::Error> {
        Uuid::parse_str(value).map(Self)
    }
}

impl From<Uuid> for SessionId {
    fn from(value: Uuid) -> Self {
        Self(value)
    }
}

impl FromStr for SessionId {
    type Err = uuid::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Uuid::parse_str(s).map(Self)
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl Display for SessionId {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// 会话运行状态。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionStatus {
    /// 会话初始化中。
    Initializing,
    /// 会话已就绪，可接收请求。
    Ready,
    /// 会话处理中。
    Processing,
    /// 会话空闲。
    Idle,
    /// 会话已关闭。
    Closed,
    /// 会话出现错误。
    Error(String),
}

/// 会话元数据。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub struct Session {
    /// 会话 ID。
    pub id: SessionId,
    /// 会话状态。
    pub status: SessionStatus,
    /// 关联 Agent 名称。
    pub agent_name: String,
    /// 会话创建时间（UTC）。
    pub created_at: DateTime<Utc>,
}

impl Session {
    /// 获取会话 ID。
    pub fn id(&self) -> &SessionId {
        &self.id
    }

    /// 获取关联 Agent 名称。
    pub fn agent_name(&self) -> &str {
        &self.agent_name
    }

    /// 获取会话状态。
    pub fn status(&self) -> &SessionStatus {
        &self.status
    }
}
