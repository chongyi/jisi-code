//! Codex CLI 执行器实现。
//!
//! 该模块实现了与 OpenAI Codex CLI 的通信协议，
//! 通过 stdin/stdout 使用 JSONL (JSON Lines) 格式进行交互。

use std::path::Path;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tokio::sync::Mutex;
use tokio::task::JoinHandle;
use tracing::info;

use crate::error::OrchestratorError;
use crate::{AgentConfig, EventBroadcaster, Executor, OrchestratorEvent, Result, SessionId};

/// Codex 推理强度选项。
#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReasoningEffort {
    Low,
    Medium,
    High,
}

impl Default for ReasoningEffort {
    fn default() -> Self {
        Self::Medium
    }
}

impl std::fmt::Display for ReasoningEffort {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReasoningEffort::Low => write!(f, "low"),
            ReasoningEffort::Medium => write!(f, "medium"),
            ReasoningEffort::High => write!(f, "high"),
        }
    }
}

/// Codex 模型配置选项。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct CodexModelOptions {
    /// 模型名称（如 "o4-mini", "gpt-4o" 等）。
    pub model: Option<String>,
    /// 推理强度（仅适用于支持推理的模型）。
    pub reasoning_effort: Option<ReasoningEffort>,
}

impl Default for CodexModelOptions {
    fn default() -> Self {
        Self {
            model: None,
            reasoning_effort: None,
        }
    }
}

struct CodexProcess {
    child: Arc<Mutex<Child>>,
    stdin: Arc<Mutex<ChildStdin>>,
    read_task: JoinHandle<()>,
    shutdown: Arc<AtomicBool>,
}

/// Codex CLI 执行器。
///
/// 通过 `codex` CLI 的 stdin/stdout 使用 JSONL 协议进行交互。
/// 支持模型选择和推理强度配置。
pub struct CodexExecutor {
    name: String,
    config: AgentConfig,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
    process: Option<CodexProcess>,
    model_options: CodexModelOptions,
}

/// Codex JSON-RPC 请求。
#[derive(Debug, Serialize)]
struct JsonRpcRequest {
    jsonrpc: &'static str,
    id: String,
    method: String,
    params: Value,
}

/// Codex JSON-RPC 响应/通知。
#[derive(Debug, Deserialize)]
struct JsonRpcMessage {
    #[serde(default)]
    jsonrpc: Option<String>,
    #[serde(default)]
    id: Option<String>,
    #[serde(default)]
    method: Option<String>,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<JsonRpcError>,
}

#[derive(Debug, Deserialize)]
struct JsonRpcError {
    #[serde(default)]
    code: i32,
    #[serde(default)]
    message: String,
}

impl CodexExecutor {
    /// 创建新的 Codex 执行器。
    pub fn new(config: AgentConfig, event_tx: Arc<EventBroadcaster>) -> Result<Self> {
        Ok(Self {
            name: config.id.clone(),
            config,
            event_tx,
            session_id: SessionId::new(),
            process: None,
            model_options: CodexModelOptions::default(),
        })
    }

    /// 使用指定模型选项创建执行器。
    pub fn with_model_options(
        config: AgentConfig,
        event_tx: Arc<EventBroadcaster>,
        model_options: CodexModelOptions,
    ) -> Result<Self> {
        Ok(Self {
            name: config.id.clone(),
            config,
            event_tx,
            session_id: SessionId::new(),
            process: None,
            model_options,
        })
    }

    fn effective_args(&self) -> Vec<String> {
        let mut args = if self.config.args.is_empty() {
            vec!["exec".to_string(), "--json".to_string()]
        } else {
            self.config.args.clone()
        };

        // 添加模型选项
        if let Some(ref model) = self.model_options.model {
            args.push("-c".to_string());
            args.push(format!("model={}", model));
        }

        // 添加推理强度
        if let Some(ref effort) = self.model_options.reasoning_effort {
            args.push("-c".to_string());
            args.push(format!("reasoning.effort={}", effort));
        }

        args
    }

    async fn send_json(stdin: &Arc<Mutex<ChildStdin>>, payload: &Value) -> Result<()> {
        let line = serde_json::to_string(payload)?;
        let mut guard = stdin.lock().await;
        guard.write_all(line.as_bytes()).await?;
        guard.write_all(b"\n").await?;
        guard.flush().await?;
        Ok(())
    }

    async fn send_user_message(stdin: &Arc<Mutex<ChildStdin>>, prompt: &str) -> Result<()> {
        // Codex 使用简单的文本输入格式
        let mut guard = stdin.lock().await;
        guard.write_all(prompt.as_bytes()).await?;
        guard.write_all(b"\n").await?;
        guard.flush().await?;
        Ok(())
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

    fn emit_file_change(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        path: String,
        action: String,
        content: Option<String>,
        diff: Option<String>,
    ) {
        event_tx.emit(OrchestratorEvent::FileChange {
            session_id: session_id.clone(),
            path,
            action,
            content,
            diff,
        });
    }

    async fn handle_jsonrpc_message(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        message: JsonRpcMessage,
    ) {
        // 处理通知消息
        if let Some(method) = &message.method {
            match method.as_str() {
                "turn/started" | "turn/completed" => {
                    // Turn 生命周期事件
                    if let Some(params) = &message.params {
                        info!(method = %method, params = ?params, "Codex turn event");
                    }
                }
                "turn/plan/updated" => {
                    // 计划更新
                    if let Some(params) = &message.params {
                        if let Some(explanation) = params.get("explanation").and_then(Value::as_str)
                        {
                            Self::emit_content_delta(
                                event_tx,
                                session_id,
                                format!("[Plan] {}\n", explanation),
                            );
                        }
                    }
                }
                "turn/diff/updated" => {
                    // 文件变更 diff
                    if let Some(params) = &message.params {
                        if let Some(diff) = params.get("diff").and_then(Value::as_str) {
                            Self::emit_file_change(
                                event_tx,
                                session_id,
                                "multiple".to_string(),
                                "edit".to_string(),
                                None,
                                Some(diff.to_string()),
                            );
                        }
                    }
                }
                "item/started" | "item/completed" => {
                    // Item 生命周期事件
                    if let Some(params) = &message.params {
                        if let Some(item) = params.get("item") {
                            if let Some(item_type) = item.get("type").and_then(Value::as_str) {
                                match item_type {
                                    "tool_call" => {
                                        let tool_name = item
                                            .get("tool_name")
                                            .and_then(Value::as_str)
                                            .unwrap_or("unknown");
                                        let args = item
                                            .get("arguments")
                                            .cloned()
                                            .unwrap_or_else(|| json!({}));
                                        Self::emit_tool_call(
                                            event_tx,
                                            session_id,
                                            tool_name.to_string(),
                                            args,
                                        );
                                    }
                                    "file_change" => {
                                        let path = item
                                            .get("path")
                                            .and_then(Value::as_str)
                                            .unwrap_or("unknown");
                                        let action = item
                                            .get("action")
                                            .and_then(Value::as_str)
                                            .unwrap_or("unknown");
                                        let content = item
                                            .get("content")
                                            .and_then(Value::as_str)
                                            .map(|s| s.to_string());
                                        let diff = item
                                            .get("diff")
                                            .and_then(Value::as_str)
                                            .map(|s| s.to_string());
                                        Self::emit_file_change(
                                            event_tx,
                                            session_id,
                                            path.to_string(),
                                            action.to_string(),
                                            content,
                                            diff,
                                        );
                                    }
                                    "message" => {
                                        if let Some(content) =
                                            item.get("content").and_then(Value::as_str)
                                        {
                                            Self::emit_content_delta(
                                                event_tx,
                                                session_id,
                                                content.to_string(),
                                            );
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                "thread/tokenUsage/updated" => {
                    // Token 使用更新
                    if let Some(params) = &message.params {
                        event_tx.emit(OrchestratorEvent::TokenUsage {
                            session_id: session_id.clone(),
                            usage: params.clone(),
                        });
                    }
                }
                _ => {
                    // 未处理的方法
                    info!(method = %method, "Unhandled Codex notification");
                }
            }
        }

        // 处理响应消息
        if let Some(result) = &message.result {
            if let Some(content) = result.get("content").and_then(Value::as_str) {
                Self::emit_content_delta(event_tx, session_id, content.to_string());
            }
        }

        // 处理错误
        if let Some(error) = &message.error {
            event_tx.emit(OrchestratorEvent::SessionError {
                session_id: session_id.clone(),
                error: format!("Codex error ({}): {}", error.code, error.message),
            });
        }
    }

    async fn read_loop(
        stdout: ChildStdout,
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

                    // 尝试解析为 JSON-RPC 消息
                    if let Ok(message) = serde_json::from_str::<JsonRpcMessage>(trimmed) {
                        Self::handle_jsonrpc_message(&event_tx, &session_id, message).await;
                    } else {
                        // 非 JSON 输出，作为普通文本处理
                        Self::emit_content_delta(&event_tx, &session_id, format!("{}\n", trimmed));
                    }
                }
                Ok(None) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        event_tx.emit(OrchestratorEvent::SessionError {
                            session_id: session_id.clone(),
                            error: "Codex process terminated".to_string(),
                        });
                    }
                    break;
                }
                Err(err) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        event_tx.emit(OrchestratorEvent::SessionError {
                            session_id: session_id.clone(),
                            error: format!("failed to read Codex output: {err}"),
                        });
                    }
                    break;
                }
            }
        }
    }
}

#[async_trait::async_trait]
impl Executor for CodexExecutor {
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
            model = ?self.model_options.model,
            reasoning_effort = ?self.model_options.reasoning_effort,
            "starting Codex executor"
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
            OrchestratorError::Executor("failed to capture Codex stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            OrchestratorError::Executor("failed to capture Codex stdout".to_string())
        })?;

        let stdin = Arc::new(Mutex::new(stdin));
        let child = Arc::new(Mutex::new(child));
        let shutdown = Arc::new(AtomicBool::new(false));
        let read_task = tokio::spawn(Self::read_loop(
            stdout,
            self.event_tx.clone(),
            self.session_id.clone(),
            shutdown.clone(),
        ));

        self.process = Some(CodexProcess {
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
            .ok_or_else(|| OrchestratorError::Executor("Executor not started".to_string()))?;
        Self::send_user_message(&process.stdin, prompt).await
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            "shutting down Codex executor"
        );

        if let Some(process) = self.process.take() {
            process.shutdown.store(true, Ordering::SeqCst);
            process.read_task.abort();

            let mut child = process.child.lock().await;
            if let Err(err) = child.kill().await {
                if err.kind() != std::io::ErrorKind::InvalidInput {
                    return Err(err.into());
                }
            }
        }
        Ok(())
    }
}
