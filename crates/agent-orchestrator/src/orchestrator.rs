use std::path::Path;
use std::sync::Arc;

use tracing::info;

use crate::{
    AcpExecutor, AgentType, EventBroadcaster, EventStream, Executor, OrchestratorConfig,
    OrchestratorError, Result, Session, SessionId, SessionManager,
};

#[derive(Debug, Clone)]
pub struct AgentInfo {
    pub id: String,
    pub display_name: String,
    pub agent_type: AgentType,
    pub enabled: bool,
}

pub struct Orchestrator {
    config: Arc<OrchestratorConfig>,
    session_manager: Arc<SessionManager>,
    event_broadcaster: Arc<EventBroadcaster>,
}

impl Orchestrator {
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

    pub async fn create_session(&self, agent_id: &str, project_path: &Path) -> Result<Session> {
        let agent_config = self
            .config
            .agents
            .iter()
            .find(|agent| agent.id == agent_id && agent.enabled)
            .cloned()
            .ok_or_else(|| OrchestratorError::AgentNotFound(agent_id.to_string()))?;

        info!(
            agent_id = %agent_config.id,
            agent_type = ?agent_config.agent_type,
            project_path = %project_path.display(),
            "creating orchestrated session"
        );

        let executor: Box<dyn Executor> = match agent_config.agent_type {
            AgentType::Acp => Box::new(AcpExecutor::new(
                agent_config,
                self.event_broadcaster.clone(),
            )?),
            _ => return Err(OrchestratorError::UnsupportedAgentType),
        };

        self.session_manager.create_session(executor, project_path).await
    }

    pub async fn send_prompt(&self, session_id: &SessionId, prompt: &str) -> Result<()> {
        self.session_manager.send_prompt(session_id, prompt).await
    }

    pub async fn close_session(&self, session_id: &SessionId) -> Result<()> {
        self.session_manager.close_session(session_id).await
    }

    pub fn subscribe_events(&self) -> EventStream {
        self.event_broadcaster.subscribe()
    }

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

    pub async fn active_sessions(&self) -> Vec<Session> {
        self.session_manager.list_sessions().await
    }

    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        self.session_manager.get_session(session_id).await
    }
}
