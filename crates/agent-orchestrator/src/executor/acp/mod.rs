//! ACP（Agent Communication Protocol）执行器实现模块。
//!
//! 当前为占位模块，后续在此补充具体 ACP 执行器实现。

use std::path::Path;
use std::sync::Arc;

use tracing::info;

use crate::error::OrchestratorError;
use crate::{AgentConfig, EventBroadcaster, Executor, Result, SessionId};
use client::AcpClient;
use process::AcpProcess;

pub mod protocol;
pub mod process;
pub mod client;

pub struct AcpExecutor {
    name: String,
    config: AgentConfig,
    client: Option<AcpClient>,
    event_tx: Arc<EventBroadcaster>,
    session_id: SessionId,
}

impl AcpExecutor {
    pub fn new(config: AgentConfig, event_tx: Arc<EventBroadcaster>) -> Result<Self> {
        Ok(Self {
            name: config.id.clone(),
            config,
            client: None,
            event_tx,
            session_id: SessionId::new(),
        })
    }

    pub fn session_id(&self) -> &SessionId {
        &self.session_id
    }
}

#[async_trait::async_trait]
impl Executor for AcpExecutor {
    fn name(&self) -> &str {
        &self.name
    }

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
        client.initialize().await?;
        self.client = Some(client);
        Ok(())
    }

    async fn send_message(&mut self, prompt: &str) -> Result<()> {
        let client = self
            .client
            .as_ref()
            .ok_or_else(|| OrchestratorError::Executor("执行器未启动".to_string()))?;
        client.send_message(prompt).await
    }

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
