use std::collections::HashMap;
use std::process::ExitStatus;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use serde_json::{Value, json};
use tokio::sync::{RwLock, oneshot};
use tracing::{debug, info, warn};

use crate::error::{OrchestratorError, Result};
use crate::events::{EventBroadcaster, OrchestratorEvent};
use crate::executor::acp::process::AcpProcess;
use crate::executor::acp::protocol::{
    JsonRpcNotification, JsonRpcRequest, JsonRpcResponse, METHOD_INITIALIZE, METHOD_SEND_MESSAGE,
    NOTIF_CONTENT_DELTA, NOTIF_STATUS, NOTIF_TOOL_CALL,
};
use crate::session::SessionId;

pub struct AcpClient {
    process: Arc<RwLock<AcpProcess>>,
    next_id: AtomicU64,
    pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
}

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);

impl AcpClient {
    pub fn new(process: AcpProcess, event_tx: Arc<EventBroadcaster>, session_id: SessionId) -> Self {
        let process = Arc::new(RwLock::new(process));
        let pending_requests = Arc::new(RwLock::new(HashMap::new()));

        let read_process = Arc::clone(&process);
        let read_pending = Arc::clone(&pending_requests);
        let read_event_tx = Arc::clone(&event_tx);
        let read_session_id = session_id.clone();

        tokio::spawn(async move {
            Self::read_loop(read_process, read_pending, read_event_tx, read_session_id).await;
        });

        Self {
            process,
            next_id: AtomicU64::new(1),
            pending_requests,
            event_tx,
            session_id,
        }
    }

    pub async fn initialize(&self) -> Result<()> {
        info!(session_id = %self.session_id, "initializing ACP client");
        let _ = self.send_request(METHOD_INITIALIZE, None).await?;
        Ok(())
    }

    pub async fn send_message(&self, prompt: &str) -> Result<()> {
        let params = json!({ "message": prompt });
        let _ = self.send_request(METHOD_SEND_MESSAGE, Some(params)).await?;
        Ok(())
    }

    pub async fn shutdown(&self) -> Result<()> {
        info!(session_id = %self.session_id, "shutting down ACP client");
        let mut process = self.process.write().await;
        process.kill().await?;
        self.event_tx.emit(OrchestratorEvent::SessionClosed {
            session_id: self.session_id.clone(),
        });
        Ok(())
    }

    async fn send_request(&self, method: &str, params: Option<Value>) -> Result<JsonRpcResponse> {
        let id = self.next_id.fetch_add(1, Ordering::Relaxed);
        let request = JsonRpcRequest::new(id, method, params);
        let payload = serde_json::to_string(&request)?;
        let (tx, rx) = oneshot::channel();

        {
            let mut pending = self.pending_requests.write().await;
            pending.insert(id, tx);
        }

        let send_result = {
            let mut process = self.process.write().await;
            process.send_line(&payload).await
        };

        if let Err(err) = send_result {
            let mut pending = self.pending_requests.write().await;
            pending.remove(&id);
            return Err(err);
        }

        let response = match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(response) => response.map_err(|_| {
                OrchestratorError::Executor(format!(
                    "ACP request cancelled before response: id={id}"
                ))
            })?,
            Err(_) => {
                let mut pending = self.pending_requests.write().await;
                pending.remove(&id);
                return Err(OrchestratorError::Executor(format!(
                    "ACP request timed out waiting for response: id={id}"
                )));
            }
        };

        if let Some(error) = response.error.as_ref() {
            return Err(OrchestratorError::Executor(format!(
                "ACP request failed: id={id}, code={}, message={}",
                error.code, error.message
            )));
        }

        Ok(response)
    }

    async fn read_loop(
        process: Arc<RwLock<AcpProcess>>,
        pending_requests: Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
        event_tx: Arc<EventBroadcaster>,
        session_id: SessionId,
    ) {
        loop {
            let line_result = {
                let mut process = process.write().await;
                process.read_line().await
            };

            let line = match line_result {
                Ok(Some(line)) => line,
                Ok(None) => {
                    let exit_info = Self::process_exit_info(&process).await;
                    event_tx.emit(OrchestratorEvent::SessionError {
                        session_id: session_id.clone(),
                        error: format!("ACP process terminated: {exit_info}"),
                    });
                    break;
                }
                Err(err) => {
                    let exit_info = Self::process_exit_info(&process).await;
                    warn!(error = %err, "failed to read ACP process output");
                    event_tx.emit(OrchestratorEvent::SessionError {
                        session_id: session_id.clone(),
                        error: format!("failed to read ACP process output: {err}; {exit_info}"),
                    });
                    break;
                }
            };

            if let Ok(response) = serde_json::from_str::<JsonRpcResponse>(&line) {
                Self::handle_response(&pending_requests, response).await;
                continue;
            }

            if let Ok(notification) = serde_json::from_str::<JsonRpcNotification>(&line) {
                Self::handle_notification(&event_tx, &session_id, notification);
                continue;
            }

            warn!(line = %line, "received unrecognized ACP payload");
        }
    }

    async fn process_exit_info(process: &Arc<RwLock<AcpProcess>>) -> String {
        let mut proc = process.write().await;
        match proc.try_wait() {
            Ok(Some(status)) => format!("process exited with {}", Self::format_exit_status(status)),
            Ok(None) => "process stdout closed but process is still running".to_string(),
            Err(err) => format!("failed to check process status: {err}"),
        }
    }

    fn format_exit_status(status: ExitStatus) -> String {
        if let Some(code) = status.code() {
            return format!("exit code {code}");
        }

        #[cfg(unix)]
        {
            use std::os::unix::process::ExitStatusExt;

            if let Some(signal) = status.signal() {
                return format!("signal {signal}");
            }
        }

        format!("status {status}")
    }

    async fn handle_response(
        pending_requests: &Arc<RwLock<HashMap<u64, oneshot::Sender<JsonRpcResponse>>>>,
        response: JsonRpcResponse,
    ) {
        let response_id = response.id;
        let tx = {
            let mut pending = pending_requests.write().await;
            pending.remove(&response_id)
        };

        match tx {
            Some(tx) => {
                if tx.send(response).is_err() {
                    debug!(id = response_id, "request receiver dropped before response delivery");
                }
            }
            None => {
                warn!(id = response_id, "received response for unknown request id");
            }
        }
    }

    fn handle_notification(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        notification: JsonRpcNotification,
    ) {
        match notification.method.as_str() {
            NOTIF_CONTENT_DELTA => {
                let content = notification
                    .params
                    .as_ref()
                    .and_then(|params| params.get("content"))
                    .and_then(|value| value.as_str())
                    .unwrap_or_default()
                    .to_string();

                event_tx.emit(OrchestratorEvent::ContentDelta {
                    session_id: session_id.clone(),
                    content,
                });
            }
            NOTIF_TOOL_CALL => {
                let (tool_name, args) = match notification.params {
                    Some(params) => {
                        let tool_name = params
                            .get("tool_name")
                            .or_else(|| params.get("toolName"))
                            .and_then(|value| value.as_str())
                            .unwrap_or("unknown")
                            .to_string();
                        let args = params
                            .get("args")
                            .cloned()
                            .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                        (tool_name, args)
                    }
                    None => ("unknown".to_string(), Value::Object(serde_json::Map::new())),
                };

                event_tx.emit(OrchestratorEvent::ToolCall {
                    session_id: session_id.clone(),
                    tool_name,
                    args,
                });
            }
            NOTIF_STATUS => {
                debug!(params = ?notification.params, "received ACP status notification");
            }
            _ => {
                debug!(
                    method = %notification.method,
                    params = ?notification.params,
                    "received unsupported ACP notification"
                );
            }
        }
    }
}
