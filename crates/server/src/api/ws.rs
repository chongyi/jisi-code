//! WebSocket API 处理器包装。
//!
//! 将统一的应用状态适配到 agent_orchestrator 的 WebSocket handler。

use std::sync::Arc;

use axum::extract::State;
use axum::extract::ws::{WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::mpsc;
use tracing::{error, info, warn};
use uuid::Uuid;

use super::state::AppState;
use agent_orchestrator::session::SessionId;
use agent_orchestrator::ws_api::{
    AgentInfoMessage, ClientMessage, ServerMessage, SessionInfoMessage, event_to_server_message,
};
use std::path::PathBuf;

/// Axum WebSocket 升级 handler，使用统一的 AppState。
pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    info!("new WebSocket connection request");
    ws.on_upgrade(move |socket| handle_socket(socket, state.orchestrator.clone()))
}

async fn handle_socket(socket: WebSocket, orchestrator: Arc<agent_orchestrator::Orchestrator>) {
    let (mut sender, mut receiver) = socket.split();
    let (out_tx, mut out_rx) = mpsc::channel::<ServerMessage>(64);
    info!("WebSocket connection established");

    let writer_task = tokio::spawn(async move {
        while let Some(server_msg) = out_rx.recv().await {
            match serde_json::to_string(&server_msg) {
                Ok(json) => {
                    info!(payload = %json, "sending WebSocket response");
                    if sender
                        .send(axum::extract::ws::Message::Text(json.into()))
                        .await
                        .is_err()
                    {
                        break;
                    }
                }
                Err(err) => {
                    error!(error = %err, "failed to serialize WebSocket message");
                    break;
                }
            }
        }
    });

    let mut event_stream = orchestrator.subscribe_events();
    let event_tx = out_tx.clone();
    let event_task = tokio::spawn(async move {
        loop {
            match event_stream.recv().await {
                Ok(event) => {
                    let msg = event_to_server_message(event);
                    if event_tx.send(msg).await.is_err() {
                        break;
                    }
                }
                Err(err) => {
                    warn!(error = %err, "event stream receive failed");
                    let _ = event_tx
                        .send(ServerMessage::Error {
                            message: format!("event stream error: {err}"),
                        })
                        .await;
                    break;
                }
            }
        }
    });

    while let Some(msg) = receiver.next().await {
        match msg {
            Ok(axum::extract::ws::Message::Text(text)) => {
                info!(payload = %text, "received WebSocket client message");
                match serde_json::from_str::<ClientMessage>(&text) {
                    Ok(client_msg) => {
                        if let Some(response) =
                            handle_client_message(&orchestrator, client_msg).await
                            && out_tx.send(response).await.is_err()
                        {
                            break;
                        }
                    }
                    Err(err) => {
                        if out_tx
                            .send(ServerMessage::Error {
                                message: format!("invalid message: {err}"),
                            })
                            .await
                            .is_err()
                        {
                            break;
                        }
                    }
                }
            }
            Ok(axum::extract::ws::Message::Close(_)) => break,
            Ok(_) => {}
            Err(err) => {
                warn!(error = %err, "WebSocket receive error");
                break;
            }
        }
    }

    event_task.abort();
    drop(out_tx);
    if let Err(err) = writer_task.await {
        warn!(error = %err, "WebSocket writer task exited with join error");
    }

    info!("WebSocket connection closed");
}

async fn handle_client_message(
    orchestrator: &agent_orchestrator::Orchestrator,
    msg: ClientMessage,
) -> Option<ServerMessage> {
    match msg {
        ClientMessage::CreateSession {
            agent_id,
            project_path,
            model_config,
        } => match orchestrator
            .create_session(&agent_id, &PathBuf::from(&project_path), model_config)
            .await
        {
            Ok(_) => None,
            Err(err) => Some(ServerMessage::Error {
                message: format!("create session failed: {err}"),
            }),
        },
        ClientMessage::SendPrompt { session_id, prompt } => {
            let sid = match Uuid::parse_str(&session_id) {
                Ok(uuid) => SessionId::from(uuid),
                Err(err) => {
                    return Some(ServerMessage::Error {
                        message: format!("invalid session_id: {err}"),
                    });
                }
            };

            match orchestrator.send_prompt(&sid, &prompt).await {
                Ok(()) => Some(ServerMessage::PromptAccepted { session_id }),
                Err(err) => Some(ServerMessage::Error {
                    message: format!("send prompt failed: {err}"),
                }),
            }
        }
        ClientMessage::CloseSession { session_id } => {
            let sid = match Uuid::parse_str(&session_id) {
                Ok(uuid) => SessionId::from(uuid),
                Err(err) => {
                    return Some(ServerMessage::Error {
                        message: format!("invalid session_id: {err}"),
                    });
                }
            };

            match orchestrator.close_session(&sid).await {
                Ok(()) => None,
                Err(err) => Some(ServerMessage::Error {
                    message: format!("close session failed: {err}"),
                }),
            }
        }
        ClientMessage::ListAgents => {
            info!("handling list_agents message");
            let agents = orchestrator.available_agents();
            info!(count = agents.len(), "prepared agent_list response");
            Some(ServerMessage::AgentList {
                agents: agents
                    .into_iter()
                    .map(|agent| AgentInfoMessage {
                        id: agent.id,
                        display_name: agent.display_name,
                        agent_type: format!("{:?}", agent.agent_type),
                        enabled: agent.enabled,
                    })
                    .collect(),
            })
        }
        ClientMessage::ListSessions => {
            let sessions = orchestrator.active_sessions().await;
            Some(ServerMessage::SessionList {
                sessions: sessions
                    .into_iter()
                    .map(|session| SessionInfoMessage {
                        session_id: session.id().to_string(),
                        agent_name: session.agent_name().to_string(),
                        status: format!("{:?}", session.status()),
                        model_config: session.model_config.clone(),
                    })
                    .collect(),
            })
        }
    }
}
