//! ACP（Agent Communication Protocol）执行器实现模块。
//!
//! 当前为占位模块，后续在此补充具体 ACP 执行器实现。

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tracing::info;

use crate::error::OrchestratorError;
use crate::{AgentConfig, EventBroadcaster, Executor, Result, SessionId};
use client::AcpClient;
use process::AcpProcess;

/// ACP 协议对象定义。
pub mod protocol;
/// ACP 子进程封装。
pub mod process;
/// ACP 客户端实现。
pub mod client;

const INIT_TIMEOUT: Duration = Duration::from_secs(30);
const SEND_MESSAGE_TIMEOUT: Duration = Duration::from_secs(60);

/// 基于 ACP 协议的执行器实现。
pub struct AcpExecutor {
    name: String,
    config: AgentConfig,
    client: Option<AcpClient>,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
}

impl AcpExecutor {
    /// 创建 ACP 执行器实例。
    pub fn new(config: AgentConfig, event_tx: Arc<EventBroadcaster>) -> Result<Self> {
        Ok(Self {
            name: config.id.clone(),
            config,
            client: None,
            event_tx,
            session_id: SessionId::new(),
        })
    }

    /// 获取执行器关联的会话 ID。
    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

#[async_trait::async_trait]
impl Executor for AcpExecutor {
    fn name(&self) -> &str {
        &self.name
    }

    #[tracing::instrument(skip(self))]
    async fn start(&mut self, project_path: &Path) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            project_path = %project_path.display(),
            "starting ACP executor"
        );

        let env_vars: Vec<(String, String)> = self
            .config
            .env
            .iter()
            .map(|e| (e.key.clone(), e.value.clone()))
            .collect();

        let process = AcpProcess::spawn(
            &self.config.command,
            &self.config.args,
            project_path,
            &env_vars,
        )
        .await?;

        let client = AcpClient::new(process, self.event_tx.clone(), self.session_id.clone());
        tokio::time::timeout(INIT_TIMEOUT, client.initialize())
            .await
            .map_err(|_| OrchestratorError::Executor("ACP initialization timed out".to_string()))??;
        self.client = Some(client);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    async fn send_message(&mut self, prompt: &str) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| OrchestratorError::Executor("执行器未启动".to_string()))?;
        tokio::time::timeout(SEND_MESSAGE_TIMEOUT, client.send_message(prompt))
            .await
            .map_err(|_| OrchestratorError::Executor("send_message timed out".to_string()))?
    }

    #[tracing::instrument(skip(self))]
    async fn shutdown(&mut self) -> Result<()> {
        info!(
            executor = %self.name,
            session_id = %self.session_id,
            "shutting down ACP executor"
        );

        if let Some(client) = self.client.as_ref() {
            client.shutdown().await?;
        }
        self.client = None;
        Ok(())
    }
}
