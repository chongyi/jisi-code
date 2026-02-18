use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::{info, warn};
use uuid::Uuid;

use crate::error::OrchestratorError;
use crate::{AgentConfig, EventBroadcaster, Executor, OrchestratorEvent, Result, SessionId};

const DEFAULT_CLAUDE_ARGS: &[&str] = &[
    "-p",
    "--verbose",
    "--output-format=stream-json",
    "--input-format=stream-json",
    "--include-partial-messages",
    "--replay-user-messages",
    "--permission-prompt-tool=stdio",
    "--permission-mode=bypassPermissions",
];

struct ClaudeSdkProcess {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    read_task: JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
}

/// Claude Agent SDK executor over `claude -p --input/output-format stream-json`.
pub struct ClaudeSdkExecutor {
    name: String,
    config: AgentConfig,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
    process: Option<ClaudeSdkProcess>,
}

impl ClaudeSdkExecutor {
    pub fn new(config: AgentConfig, event_tx: Arc<EventBroadcaster>) -> Result<Self> {
        Ok(Self {
            name: config.id.clone(),
            config,
            event_tx,
            session_id: SessionId::new(),
            process: None,
        })
    }

    fn effective_args(&self) -> Vec<String> {
        if self.config.args.is_empty() {
            return DEFAULT_CLAUDE_ARGS
                .iter()
                .map(|arg| (*arg).to_string())
                .collect();
        }
        self.config.args.clone()
    }

    async fn send_json(stdin: &Arc<Mutex<ChildStdin>>, payload: &Value) -> Result<()> {
        let line = serde_json::to_string(payload)?;
        let mut guard = stdin.lock().await;
        guard.write_all(line.as_bytes()).await?;
        guard.write_all(b"\n").await?;
        guard.flush().await?;
        Ok(())
    }

    async fn send_initialize(stdin: &Arc<Mutex<ChildStdin>>) -> Result<()> {
        Self::send_json(
            stdin,
            &json!({
                "type": "control_request",
                "request_id": Uuid::new_v4().to_string(),
                "request": {
                    "subtype": "initialize"
                }
            }),
        )
        .await
    }

    async fn send_permission_mode(stdin: &Arc<Mutex<ChildStdin>>) -> Result<()> {
        Self::send_json(
            stdin,
            &json!({
                "type": "control_request",
                "request_id": Uuid::new_v4().to_string(),
                "request": {
                    "subtype": "set_permission_mode",
                    "mode": "bypassPermissions"
                }
            }),
        )
        .await
    }

    async fn send_user_message(stdin: &Arc<Mutex<ChildStdin>>, prompt: &str) -> Result<()> {
        Self::send_json(
            stdin,
            &json!({
                "type": "user",
                "message": {
                    "role": "user",
                    "content": prompt
                }
            }),
        )
        .await
    }

    async fn send_control_success(
        stdin: &Arc<Mutex<ChildStdin>>,
        request_id: &str,
        response: Value,
    ) -> Result<()> {
        Self::send_json(
            stdin,
            &json!({
                "type": "control_response",
                "response": {
                    "subtype": "success",
                    "request_id": request_id,
                    "response": response
                }
            }),
        )
        .await
    }

    async fn send_control_error(
        stdin: &Arc<Mutex<ChildStdin>>,
        request_id: &str,
        error: &str,
    ) -> Result<()> {
        Self::send_json(
            stdin,
            &json!({
                "type": "control_response",
                "response": {
                    "subtype": "error",
                    "request_id": request_id,
                    "error": error
                }
            }),
        )
        .await
    }

    async fn handle_control_request(stdin: &Arc<Mutex<ChildStdin>>, payload: &Value) -> Result<()> {
        let request_id = payload
            .get("request_id")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let request = payload.get("request").cloned().unwrap_or_else(|| json!({}));
        let subtype = request
            .get("subtype")
            .and_then(Value::as_str)
            .unwrap_or_default();

        match subtype {
            "can_use_tool" => {
                let input = request.get("input").cloned().unwrap_or_else(|| json!({}));
                Self::send_control_success(
                    stdin,
                    request_id,
                    json!({
                        "behavior": "allow",
                        "updatedInput": input
                    }),
                )
                .await
            }
            "hook_callback" => {
                Self::send_control_success(
                    stdin,
                    request_id,
                    json!({
                        "hookSpecificOutput": {
                            "hookEventName": "PreToolUse",
                            "permissionDecision": "allow",
                            "permissionDecisionReason": "Approved by orchestrator"
                        }
                    }),
                )
                .await
            }
            other => {
                Self::send_control_error(
                    stdin,
                    request_id,
                    &format!("unsupported control request subtype: {other}"),
                )
                .await
            }
        }
    }

    fn emit_content_delta(event_tx: &EventBroadcaster, session_id: &SessionId, text: String) {
        if text.is_empty() {
            return;
        }
        event_tx.emit(OrchestratorEvent::ContentDelta {
            session_id: session_id.clone(),
            content: text,
        });
    }

    fn emit_tool_call(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        tool_name: String,
        args: Value,
    ) {
        event_tx.emit(OrchestratorEvent::ToolCall {
            session_id: session_id.clone(),
            tool_name,
            args,
        });
    }

    async fn handle_stream_event(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        payload: &Value,
    ) {
        let event = match payload.get("event") {
            Some(Value::Object(_)) => payload.get("event").unwrap_or(&Value::Null),
            _ => return,
        };

        let event_type = event
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match event_type {
            "content_block_delta" => {
                let text = event
                    .get("delta")
                    .and_then(|delta| {
                        if delta.get("type").and_then(Value::as_str) == Some("text_delta") {
                            delta.get("text").and_then(Value::as_str)
                        } else {
                            None
                        }
                    })
                    .unwrap_or_default()
                    .to_string();
                Self::emit_content_delta(event_tx, session_id, text);
            }
            "content_block_start" => {
                let content_block = event.get("content_block").unwrap_or(&Value::Null);
                if content_block.get("type").and_then(Value::as_str) == Some("tool_use") {
                    let tool_name = content_block
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string();
                    let args = content_block
                        .get("input")
                        .cloned()
                        .unwrap_or_else(|| Value::Object(serde_json::Map::new()));
                    Self::emit_tool_call(event_tx, session_id, tool_name, args);
                }
            }
            _ => {}
        }
    }

    async fn read_loop(
        stdout: ChildStdout,
        stdin: Arc<Mutex<ChildStdin>>,
        event_tx: Arc<EventBroadcaster>,
        session_id: SessionId,
        shutdown: Arc<AtomicBool>,
    ) {
        let mut reader = BufReader::new(stdout).lines();

        loop {
            match reader.next_line().await {
                Ok(Some(line)) => {
                    let trimmed = line.trim();
                    if trimmed.is_empty() {
                        continue;
                    }

                    let payload = match serde_json::from_str::<Value>(trimmed) {
                        Ok(value) => value,
                        Err(_) => {
                            warn!(line = %trimmed, "received non-JSON Claude SDK output");
                            continue;
                        }
                    };

                    let msg_type = payload
                        .get("type")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    match msg_type {
                        "control_request" => {
                            if let Err(err) = Self::handle_control_request(&stdin, &payload).await {
                                event_tx.emit(OrchestratorEvent::SessionError {
                                    session_id: session_id.clone(),
                                    error: format!(
                                        "failed to respond to Claude control request: {err}"
                                    ),
                                });
                            }
                        }
                        "stream_event" => {
                            Self::handle_stream_event(&event_tx, &session_id, &payload).await;
                        }
                        "result" => {
                            let is_error =
                                payload.get("is_error").and_then(Value::as_bool) == Some(true);
                            if is_error {
                                let error = payload
                                    .get("error")
                                    .and_then(Value::as_str)
                                    .unwrap_or("Claude result reported error")
                                    .to_string();
                                event_tx.emit(OrchestratorEvent::SessionError {
                                    session_id: session_id.clone(),
                                    error,
                                });
                            }
                        }
                        _ => {}
                    }
                }
                Ok(None) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        event_tx.emit(OrchestratorEvent::SessionError {
                            session_id: session_id.clone(),
                            error: "Claude SDK process terminated".to_string(),
                        });
                    }
                    break;
                }
                Err(err) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        event_tx.emit(OrchestratorEvent::SessionError {
                            session_id: session_id.clone(),
                            error: format!("failed to read Claude SDK output: {err}"),
                        });
                    }
                    break;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Executor for ClaudeSdkExecutor {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.session_id = session_id;
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self, project_path: &Path) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            project_path = %project_path.display(),
            "starting Claude SDK executor"
        );

        let mut command = Command::new(&self.config.command);
        command
            .args(self.effective_args())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .current_dir(project_path);

        for env_var in &self.config.env {
            command.env(&env_var.key, &env_var.value);
        }

        let mut child = command.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            OrchestratorError::Executor("failed to capture Claude SDK stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            OrchestratorError::Executor("failed to capture Claude SDK stdout".to_string())
        })?;

        let stdin = Arc::new(Mutex::new(stdin));
        let child = Arc::new(Mutex::new(child));
        let shutdown = Arc::new(AtomicBool::new(false));
        let read_task = tokio::spawn(Self::read_loop(
            stdout,
            stdin.clone(),
            self.event_tx.clone(),
            self.session_id.clone(),
            shutdown.clone(),
        ));

        Self::send_initialize(&stdin).await?;
        Self::send_permission_mode(&stdin).await?;

        self.process = Some(ClaudeSdkProcess {
            child,
            stdin,
            read_task,
            shutdown,
        });
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn send_message(&mut self, prompt: &str) -> Result<()> {
        let process = self
            .process
            .as_ref()
            .ok_or_else(|| OrchestratorError::Executor("执行器未启动".to_string()))?;
        Self::send_user_message(&process.stdin, prompt).await
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            "shutting down Claude SDK executor"
        );

        if let Some(process) = self.process.take() {
            process.shutdown.store(true, Ordering::SeqCst);
            process.read_task.abort();

            let mut child = process.child.lock().await;
            if let Err(err) = child.kill().await {
                // Ignore "already exited" kill errors.
                if err.kind() != std::io::ErrorKind::InvalidInput {
                    return Err(err.into());
                }
            }
        }
        Ok(())
    }
}
