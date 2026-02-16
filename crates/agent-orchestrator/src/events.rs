use anyhow::Result;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast;
use crate::session::SessionId;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum OrchestratorEvent {
    SessionCreated {
        session_id: SessionId,
        agent_name: String,
    },
    ContentDelta {
        session_id: SessionId,
        content: String,
    },
    ToolCall {
        session_id: SessionId,
        tool_name: String,
        args: serde_json::Value,
    },
    SessionError {
        session_id: SessionId,
        error: String,
    },
    SessionClosed {
        session_id: SessionId,
    },
}

#[derive(Debug, Clone)]
pub struct EventBroadcaster {
    sender: broadcast::Sender<OrchestratorEvent>,
}

impl EventBroadcaster {
    pub fn new(capacity: usize) -> Self {
        let (sender, _) = broadcast::channel(capacity);
        Self { sender }
    }

    pub fn emit(&self, event: OrchestratorEvent) {
        let _ = self.sender.send(event);
    }

    pub fn subscribe(&self) -> EventStream {
        EventStream {
            receiver: self.sender.subscribe(),
        }
    }
}

#[derive(Debug)]
pub struct EventStream {
    receiver: broadcast::Receiver<OrchestratorEvent>,
}

impl EventStream {
    pub async fn recv(&mut self) -> Result<OrchestratorEvent> {
        Ok(self.receiver.recv().await?)
    }

    pub fn try_recv(&mut self) -> Result<OrchestratorEvent> {
        Ok(self.receiver.try_recv()?)
    }
}
