use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::session::SessionId;

/// 编排器对外广播的事件类型。
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchestratorEvent {
    /// 会话创建完成事件。
    SessionCreated {
        /// 会话 ID。
        session_id: SessionId,
        /// 执行该会话的 Agent 名称。
        agent_name: String,
    },
    /// 模型增量输出事件。
    ContentDelta {
        /// 会话 ID。
        session_id: SessionId,
        /// 增量文本内容。
        content: String,
    },
    /// 工具调用事件。
    ToolCall {
        /// 会话 ID。
        session_id: SessionId,
        /// 工具名称。
        tool_name: String,
        /// 工具调用参数。
        args: serde_json::Value,
    },
    /// 会话错误事件。
    SessionError {
        /// 会话 ID。
        session_id: SessionId,
        /// 错误描述。
        error: String,
    },
    /// 会话关闭事件。
    SessionClosed {
        /// 会话 ID。
        session_id: SessionId,
    },
}

/// 基于 `tokio::broadcast` 的事件广播器。
#[derive(Debug, Clone)]
pub struct EventBroadcaster {
    sender: broadcast::Sender<OrchestratorEvent>,
}

impl EventBroadcaster {
    /// 创建事件广播器。
    ///
    /// `capacity` 表示内部广播队列容量。
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    /// 广播一个事件。
    pub fn emit(&self, event: OrchestratorEvent) {
        let _ = self.sender.send(event);
    }

    /// 订阅事件流。
    pub fn subscribe(&self) -> EventStream {
        EventStream {
            receiver: self.sender.subscribe(),
        }
    }
}

/// 事件接收流包装器。
#[derive(Debug)]
pub struct EventStream {
    receiver: broadcast::Receiver<OrchestratorEvent>,
}

impl EventStream {
    /// 异步接收下一条事件。
    pub async fn recv(&mut self) -> Result<OrchestratorEvent> {
        Ok(self.receiver.recv().await?)
    }

    /// 非阻塞尝试接收一条事件。
    pub fn try_recv(&mut self) -> Result<OrchestratorEvent> {
        Ok(self.receiver.try_recv()?)
    }
}
