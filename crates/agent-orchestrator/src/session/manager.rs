use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

use chrono::Utc;
use tokio::sync::RwLock;
use tracing::info;

use crate::{
    EventBroadcaster, Executor, OrchestratorError, OrchestratorEvent, Result, Session, SessionId,
    SessionStatus,
};

struct SessionState {
    session: Session,
    executor: Box<dyn Executor>,
}

/// 会话生命周期管理器。
pub struct SessionManager {
    sessions: Arc<RwLock<HashMap<SessionId, SessionState>>>,
    event_broadcaster: Arc<EventBroadcaster>,
}

impl SessionManager {
    /// 创建会话管理器。
    pub fn new(event_broadcaster: Arc<EventBroadcaster>) -> Self {
        Self {
            sessions: Arc::new(RwLock::new(HashMap::new())),
            event_broadcaster,
        }
    }

    /// 创建会话并启动执行器。
    #[tracing::instrument(skip(self, executor))]
    pub async fn create_session(
        &self,
        mut executor: Box<dyn Executor>,
        project_path: &Path,
    ) -> Result<Session> {
        let session_id = SessionId::new();
        executor.start(project_path).await?;

        let session = Session {
            id: session_id.clone(),
            status: SessionStatus::Ready,
            agent_name: executor.name().to_string(),
            created_at: Utc::now(),
        };

        info!(
            session_id = %session.id,
            agent_name = %session.agent_name,
            "creating session"
        );

        let mut sessions = self.sessions.write().await;
        sessions.insert(
            session_id.clone(),
            SessionState {
                session: session.clone(),
                executor,
            },
        );
        drop(sessions);

        self.event_broadcaster.emit(OrchestratorEvent::SessionCreated {
            session_id,
            agent_name: session.agent_name.clone(),
        });

        Ok(session)
    }

    /// 向指定会话发送提示词。
    #[tracing::instrument(skip(self))]
    pub async fn send_prompt(&self, session_id: &SessionId, prompt: &str) -> Result<()> {
        info!(session_id = %session_id, "sending prompt");

        let mut sessions = self.sessions.write().await;
        let state = sessions
            .get_mut(session_id)
            .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.to_string()))?;
        state.executor.send_message(prompt).await
    }

    /// 关闭并移除指定会话。
    #[tracing::instrument(skip(self))]
    pub async fn close_session(&self, session_id: &SessionId) -> Result<()> {
        info!(session_id = %session_id, "closing session");

        let mut state = {
            let mut sessions = self.sessions.write().await;
            sessions
                .remove(session_id)
                .ok_or_else(|| OrchestratorError::SessionNotFound(session_id.to_string()))?
        };

        state.executor.shutdown().await?;
        self.event_broadcaster.emit(OrchestratorEvent::SessionClosed {
            session_id: session_id.clone(),
        });

        Ok(())
    }

    /// 列出当前所有会话。
    pub async fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions.values().map(|state| state.session.clone()).collect()
    }

    /// 查询指定会话。
    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|state| state.session.clone())
    }
}
