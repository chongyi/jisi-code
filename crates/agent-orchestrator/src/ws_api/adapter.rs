use super::protocol::ServerMessage;
use crate::events::OrchestratorEvent;

/// 将内部 OrchestratorEvent 转换为 WebSocket ServerMessage。
pub fn event_to_server_message(event: OrchestratorEvent) -> ServerMessage {
    match event {
        OrchestratorEvent::SessionCreated {
            session_id,
            agent_name,
        } => ServerMessage::SessionCreated {
            session_id: session_id.to_string(),
            agent_name,
        },
        OrchestratorEvent::ContentDelta {
            session_id,
            content,
        } => ServerMessage::ContentDelta {
            session_id: session_id.to_string(),
            content,
        },
        OrchestratorEvent::ToolCall {
            session_id,
            tool_name,
            args,
        } => ServerMessage::ToolCall {
            session_id: session_id.to_string(),
            tool_name,
            args,
        },
        OrchestratorEvent::SessionError { session_id, error } => ServerMessage::Error {
            message: format!("session {session_id}: {error}"),
        },
        OrchestratorEvent::SessionClosed { session_id } => ServerMessage::SessionClosed {
            session_id: session_id.to_string(),
        },
    }
}
