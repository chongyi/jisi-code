use crate::session::{SessionId, SessionModelConfig};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::broadcast;

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
        /// 会话模型配置（若创建时提供）。
        model_config: Option<SessionModelConfig>,
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
        args: Value,
    },
    /// 文件变更事件。
    FileChange {
        /// 会话 ID。
        session_id: SessionId,
        /// 文件路径。
        path: String,
        /// 操作类型（read, write, edit, delete）。
        action: String,
        /// 文件内容（用于 write）。
        content: Option<String>,
        /// Diff 内容（用于 edit）。
        diff: Option<String>,
    },
    /// Token 使用信息事件。
    TokenUsage {
        /// 会话 ID。
        session_id: SessionId,
        /// Token 使用详情。
        usage: Value,
    },
    /// 思考/推理过程事件。
    Thinking {
        /// 会话 ID。
        session_id: SessionId,
        /// 思考内容。
        content: String,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_event_broadcast_and_receive() {
        let broadcaster = EventBroadcaster::new(16);
        let mut stream = broadcaster.subscribe();
        let session_id = SessionId::new();

        broadcaster.emit(OrchestratorEvent::SessionCreated {
            session_id: session_id.clone(),
            agent_name: "mock-agent".to_string(),
            model_config: None,
        });

        let event = stream.recv().await.expect("event should be received");
        match event {
            OrchestratorEvent::SessionCreated {
                session_id: received_id,
                agent_name,
                model_config,
            } => {
                assert_eq!(received_id, session_id);
                assert_eq!(agent_name, "mock-agent");
                assert!(model_config.is_none());
            }
            other => panic!("expected SessionCreated, got: {other:?}"),
        }
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let broadcaster = EventBroadcaster::new(16);
        let mut stream_a = broadcaster.subscribe();
        let mut stream_b = broadcaster.subscribe();
        let session_id = SessionId::new();

        broadcaster.emit(OrchestratorEvent::SessionClosed {
            session_id: session_id.clone(),
        });

        let event_a = stream_a.recv().await.expect("subscriber A should receive");
        let event_b = stream_b.recv().await.expect("subscriber B should receive");

        match event_a {
            OrchestratorEvent::SessionClosed {
                session_id: received_id,
            } => assert_eq!(received_id, session_id),
            other => panic!("expected SessionClosed for A, got: {other:?}"),
        }

        match event_b {
            OrchestratorEvent::SessionClosed {
                session_id: received_id,
            } => assert_eq!(received_id, session_id),
            other => panic!("expected SessionClosed for B, got: {other:?}"),
        }
    }

    #[test]
    fn test_try_recv_empty() {
        let broadcaster = EventBroadcaster::new(16);
        let mut stream = broadcaster.subscribe();
        assert!(stream.try_recv().is_err());
    }
}
