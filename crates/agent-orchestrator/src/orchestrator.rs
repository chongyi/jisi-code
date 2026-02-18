use std::path::Path;
use std::sync::Arc;

use tracing::info;

use crate::{
    AcpExecutor, AgentType, ClaudeSdkExecutor, CodexExecutor, EventBroadcaster, EventStream,
    Executor, OpenCodeExecutor, OrchestratorConfig, OrchestratorError, ReasoningEffort, Result,
    Session, SessionId, SessionManager, SessionModelConfig, SessionReasoningEffort,
};

/// 对外暴露的 Agent 元信息。
#[derive(Debug, Clone)]
pub struct AgentInfo {
    /// Agent 唯一标识。
    pub id: String,
    /// Agent 展示名称。
    pub display_name: String,
    /// Agent 类型。
    pub agent_type: AgentType,
    /// 是否启用。
    pub enabled: bool,
}

/// 编排器统一入口。
pub struct Orchestrator {
    config: Arc<OrchestratorConfig>,
    session_manager: Arc<SessionManager>,
    event_broadcaster: Arc<EventBroadcaster>,
}

impl Orchestrator {
    /// 使用给定配置创建编排器实例。
    pub fn new(config: OrchestratorConfig) -> Result<Self> {
        info!(
            event_buffer_size = config.event_buffer_size,
            agent_count = config.agents.len(),
            "initializing orchestrator"
        );

        let event_broadcaster = Arc::new(EventBroadcaster::new(config.event_buffer_size));
        let session_manager = Arc::new(SessionManager::new(event_broadcaster.clone()));

        Ok(Self {
            config: Arc::new(config),
            session_manager,
            event_broadcaster,
        })
    }

    /// 创建一个新会话并启动对应执行器。
    ///
    /// 仅允许创建已启用且存在的 `agent_id` 会话。
    #[tracing::instrument(skip(self))]
    pub async fn create_session(
        &self,
        agent_id: &str,
        project_path: &Path,
        model_config: Option<SessionModelConfig>,
    ) -> Result<Session> {
        let agent_config = self
            .config
            .agents
            .iter()
            .find(|agent| agent.id == agent_id && agent.enabled)
            .cloned()
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))?;
        let normalized_model_config = model_config.and_then(SessionModelConfig::normalized);

        info!(
            agent_id = %agent_config.id,
            agent_type = ?agent_config.agent_type,
            project_path = %project_path.display(),
            model = ?normalized_model_config.as_ref().and_then(|cfg| cfg.model.as_ref()),
            reasoning_effort = ?normalized_model_config
                .as_ref()
                .and_then(|cfg| cfg.reasoning_effort.as_ref()),
            "creating orchestrated session"
        );

        let executor: Box<dyn Executor> = match agent_config.agent_type {
            AgentType::Acp => Box::new(AcpExecutor::new(
                agent_config,
                self.event_broadcaster.clone(),
            )?),
            AgentType::ClaudeSdk => Box::new(ClaudeSdkExecutor::new(
                agent_config,
                self.event_broadcaster.clone(),
            )?),
            AgentType::Codex => {
                let options = crate::CodexModelOptions {
                    model: normalized_model_config
                        .as_ref()
                        .and_then(|cfg| cfg.model.clone()),
                    reasoning_effort: normalized_model_config
                        .as_ref()
                        .and_then(|cfg| cfg.reasoning_effort.as_ref())
                        .map(map_reasoning_effort),
                };

                if options.model.is_some() || options.reasoning_effort.is_some() {
                    Box::new(CodexExecutor::with_model_options(
                        agent_config,
                        self.event_broadcaster.clone(),
                        options,
                    )?)
                } else {
                    Box::new(CodexExecutor::new(
                        agent_config,
                        self.event_broadcaster.clone(),
                    )?)
                }
            }
            AgentType::OpenCode => {
                let options = crate::OpenCodeModelOptions {
                    model: normalized_model_config
                        .as_ref()
                        .and_then(|cfg| cfg.model.clone()),
                    provider: None,
                };

                if options.model.is_some() {
                    Box::new(OpenCodeExecutor::with_model_options(
                        agent_config,
                        self.event_broadcaster.clone(),
                        options,
                    )?)
                } else {
                    Box::new(OpenCodeExecutor::new(
                        agent_config,
                        self.event_broadcaster.clone(),
                    )?)
                }
            }
        };

        self.session_manager
            .create_session(executor, project_path, normalized_model_config)
            .await
    }

    /// 向指定会话发送用户提示词。
    #[tracing::instrument(skip(self))]
    pub async fn send_prompt(&self, session_id: &SessionId, prompt: &str) -> Result<()> {
        self.session_manager.send_prompt(session_id, prompt).await
    }

    /// 关闭指定会话并释放执行器资源。
    #[tracing::instrument(skip(self))]
    pub async fn close_session(&self, session_id: &SessionId) -> Result<()> {
        self.session_manager.close_session(session_id).await
    }

    /// 订阅编排器事件流。
    pub fn subscribe_events(&self) -> EventStream {
        self.event_broadcaster.subscribe()
    }

    /// 获取当前可用（已启用）的 Agent 列表。
    pub fn available_agents(&self) -> Vec<AgentInfo> {
        self.config
            .agents
            .iter()
            .filter(|agent| agent.enabled)
            .map(|agent| AgentInfo {
                id: agent.id.clone(),
                display_name: agent.display_name.clone(),
                agent_type: agent.agent_type,
                enabled: agent.enabled,
            })
            .collect()
    }

    /// 获取当前活跃会话列表。
    pub async fn active_sessions(&self) -> Vec<Session> {
        self.session_manager.list_sessions().await
    }

    /// 根据会话 ID 查询会话信息。
    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        self.session_manager.get_session(session_id).await
    }
}

fn map_reasoning_effort(value: &SessionReasoningEffort) -> ReasoningEffort {
    match value {
        SessionReasoningEffort::Low => ReasoningEffort::Low,
        SessionReasoningEffort::Medium => ReasoningEffort::Medium,
        SessionReasoningEffort::High => ReasoningEffort::High,
    }
}
