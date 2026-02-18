//! OpenCode 执行器实现。
//!
//! 该模块实现了与 OpenCode 的通信协议，
//! 通过 HTTP API + SSE (Server-Sent Events) 进行交互。

use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use futures_util::StreamExt;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tokio::task::JoinHandle;
use tracing::{info, warn};

use crate::error::OrchestratorError;
use crate::{AgentConfig, EventBroadcaster, Executor, OrchestratorEvent, Result, SessionId};

/// OpenCode 服务器默认端口。
const DEFAULT_PORT: u16 = 4096;

/// OpenCode 模型配置选项。
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct OpenCodeModelOptions {
    /// 模型名称。
    pub model: Option<String>,
    /// Provider 名称。
    pub provider: Option<String>,
}

impl Default for OpenCodeModelOptions {
    fn default() -> Self {
        Self {
            model: None,
            provider: None,
        }
    }
}

/// OpenCode 执行器。
///
/// 通过 HTTP API + SSE 与 OpenCode 服务器进行交互。
pub struct OpenCodeExecutor {
    name: String,
    config: AgentConfig,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
    client: Client,
    base_url: String,
    internal_session_id: Option<String>,
    sse_task: Option<JoinHandle<()>>,
    shutdown: Arc<AtomicBool>,
    model_options: OpenCodeModelOptions,
}

/// OpenCode 会话创建请求。
#[derive(Debug, Serialize)]
struct CreateSessionRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider: Option<String>,
}

/// OpenCode 会话响应。
#[derive(Debug, Deserialize)]
struct SessionResponse {
    id: String,
    #[serde(default)]
    model: Option<String>,
    #[serde(default)]
    provider: Option<String>,
}

/// OpenCode 发送提示请求。
#[derive(Debug, Serialize)]
struct PromptRequest {
    parts: Vec<PromptPart>,
}

#[derive(Debug, Serialize)]
struct PromptPart {
    #[serde(rename = "type")]
    part_type: String,
    text: String,
}

/// OpenCode 消息响应。
#[derive(Debug, Deserialize)]
struct MessageResponse {
    id: String,
    role: String,
    #[serde(default)]
    content: Option<String>,
    #[serde(default)]
    parts: Vec<MessagePart>,
}

#[derive(Debug, Deserialize)]
struct MessagePart {
    #[serde(rename = "type")]
    part_type: String,
    #[serde(default)]
    text: Option<String>,
}

/// SSE 事件结构。
#[derive(Debug, Deserialize)]
struct SseEvent {
    #[serde(default)]
    #[serde(rename = "type")]
    event_type: Option<String>,
    #[serde(flatten)]
    payload: Value,
}

impl OpenCodeExecutor {
    /// 创建新的 OpenCode 执行器。
    pub fn new(config: AgentConfig, event_tx: Arc<EventBroadcaster>) -> Result<Self> {
        let base_url = format!("http://127.0.0.1:{}", DEFAULT_PORT);
        Ok(Self {
            name: config.id.clone(),
            config,
            event_tx,
            session_id: SessionId::new(),
            client: Client::new(),
            base_url,
            internal_session_id: None,
            sse_task: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            model_options: OpenCodeModelOptions::default(),
        })
    }

    /// 使用指定模型选项创建执行器。
    pub fn with_model_options(
        config: AgentConfig,
        event_tx: Arc<EventBroadcaster>,
        model_options: OpenCodeModelOptions,
    ) -> Result<Self> {
        let base_url = format!("http://127.0.0.1:{}", DEFAULT_PORT);
        Ok(Self {
            name: config.id.clone(),
            config,
            event_tx,
            session_id: SessionId::new(),
            client: Client::new(),
            base_url,
            internal_session_id: None,
            sse_task: None,
            shutdown: Arc::new(AtomicBool::new(false)),
            model_options,
        })
    }

    async fn create_session(&mut self) -> Result<String> {
        let request = CreateSessionRequest {
            model: self.model_options.model.clone(),
            provider: self.model_options.provider.clone(),
        };

        let response = self
            .client
            .post(format!("{}/session", self.base_url))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

        let session: SessionResponse = response.json().await?;
        Ok(session.id)
    }

    async fn send_prompt_to_session(&self, session_id: &str, prompt: &str) -> Result<()> {
        let request = PromptRequest {
            parts: vec![PromptPart {
                part_type: "text".to_string(),
                text: prompt.to_string(),
            }],
        };

        self.client
            .post(format!("{}/session/{}/prompt", self.base_url, session_id))
            .json(&request)
            .send()
            .await?
            .error_for_status()?;

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

    async fn sse_loop(
        base_url: String,
        event_tx: Arc<EventBroadcaster>,
        session_id: SessionId,
        shutdown: Arc<AtomicBool>,
    ) {
        let client = Client::new();

        loop {
            if shutdown.load(Ordering::SeqCst) {
                break;
            }

            // 连接 SSE 端点
            let response = match client.get(format!("{}/event", base_url)).send().await {
                Ok(r) => r,
                Err(e) => {
                    if !shutdown.load(Ordering::SeqCst) {
                        warn!(error = %e, "Failed to connect to OpenCode SSE");
                        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;
                    }
                    continue;
                }
            };

            // 使用字节流读取
            let mut current_event_type = String::new();
            let mut current_data = String::new();

            // 将字节流转换为行流
            let mut stream = response.bytes_stream();
            let mut buffer = String::new();

            while let Some(chunk_result) = stream.next().await {
                if shutdown.load(Ordering::SeqCst) {
                    break;
                }

                let chunk = match chunk_result {
                    Ok(c) => c,
                    Err(e) => {
                        warn!(error = %e, "Error reading SSE stream");
                        break;
                    }
                };

                // 将 chunk 转换为字符串
                let text = match String::from_utf8(chunk.to_vec()) {
                    Ok(t) => t,
                    Err(_) => continue,
                };

                buffer.push_str(&text);

                // 处理缓冲区中的完整行
                while let Some(newline_pos) = buffer.find('\n') {
                    let line: String = buffer[..newline_pos].trim().to_string();
                    buffer = buffer[newline_pos + 1..].to_string();

                    if line.is_empty() {
                        // 空行表示事件结束，处理当前事件
                        if !current_data.is_empty() {
                            if let Ok(event) = serde_json::from_str::<Value>(&current_data) {
                                Self::handle_sse_event(
                                    &event_tx,
                                    &session_id,
                                    &current_event_type,
                                    &event,
                                )
                                .await;
                            }
                        }
                        current_event_type.clear();
                        current_data.clear();
                        continue;
                    }

                    if let Some(event_type) = line.strip_prefix("event:") {
                        current_event_type = event_type.trim().to_string();
                    } else if let Some(data) = line.strip_prefix("data:") {
                        current_data = data.trim().to_string();
                    }
                }
            }
        }
    }

    async fn handle_sse_event(
        event_tx: &EventBroadcaster,
        session_id: &SessionId,
        event_type: &str,
        payload: &Value,
    ) {
        match event_type {
            "message.received" | "message.processed" => {
                if let Some(content) = payload.get("content").and_then(Value::as_str) {
                    Self::emit_content_delta(event_tx, session_id, content.to_string());
                }
                if let Some(parts) = payload.get("parts").and_then(Value::as_array) {
                    for part in parts {
                        if part.get("type").and_then(Value::as_str) == Some("text") {
                            if let Some(text) = part.get("text").and_then(Value::as_str) {
                                Self::emit_content_delta(event_tx, session_id, text.to_string());
                            }
                        }
                    }
                }
            }
            "tool.call" => {
                let tool_name = payload
                    .get("tool_name")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown")
                    .to_string();
                let args = payload
                    .get("arguments")
                    .cloned()
                    .unwrap_or_else(|| json!({}));
                Self::emit_tool_call(event_tx, session_id, tool_name, args);
            }
            "error" => {
                let error_msg = payload
                    .get("message")
                    .and_then(Value::as_str)
                    .unwrap_or("Unknown error")
                    .to_string();
                event_tx.emit(OrchestratorEvent::SessionError {
                    session_id: session_id.clone(),
                    error: error_msg,
                });
            }
            "token.usage" => {
                event_tx.emit(OrchestratorEvent::TokenUsage {
                    session_id: session_id.clone(),
                    usage: payload.clone(),
                });
            }
            _ => {
                // 未处理的事件类型
                info!(event_type = %event_type, "Unhandled OpenCode SSE event");
            }
        }
    }
}

#[async_trait::async_trait]
impl Executor for OpenCodeExecutor {
    fn name(&self) -> &str {
        &self.name
    }

    fn set_session_id(&mut self, session_id: SessionId) {
        self.session_id = session_id;
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self, _project_path: &Path) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            model = ?self.model_options.model,
            provider = ?self.model_options.provider,
            "starting OpenCode executor"
        );

        // 创建 OpenCode 会话
        let internal_session_id = self.create_session().await?;
        self.internal_session_id = Some(internal_session_id.clone());

        // 启动 SSE 监听
        self.shutdown.store(false, Ordering::SeqCst);
        let sse_task = tokio::spawn(Self::sse_loop(
            self.base_url.clone(),
            self.event_tx.clone(),
            self.session_id.clone(),
            self.shutdown.clone(),
        ));
        self.sse_task = Some(sse_task);

        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn send_message(&mut self, prompt: &str) -> Result<()> {
        let session_id = self.internal_session_id.as_ref().ok_or_else(|| {
            OrchestratorError::Executor("OpenCode session not created".to_string())
        })?;

        self.send_prompt_to_session(session_id, prompt).await
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            "shutting down OpenCode executor"
        );

        self.shutdown.store(true, Ordering::SeqCst);

        if let Some(task) = self.sse_task.take() {
            task.abort();
        }

        // 可选：删除 OpenCode 会话
        if let Some(ref internal_id) = self.internal_session_id {
            let _ = self
                .client
                .delete(format!("{}/session/{}", self.base_url, internal_id))
                .send()
                .await;
        }
        self.internal_session_id = None;

        Ok(())
    }
}
