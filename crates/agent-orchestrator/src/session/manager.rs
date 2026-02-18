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
        executor.set_session_id(session_id.clone());
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

        self.event_broadcaster
            .emit(OrchestratorEvent::SessionCreated {
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
        self.event_broadcaster
            .emit(OrchestratorEvent::SessionClosed {
                session_id: session_id.clone(),
            });

        Ok(())
    }

    /// 列出当前所有会话。
    pub async fn list_sessions(&self) -> Vec<Session> {
        let sessions = self.sessions.read().await;
        sessions
            .values()
            .map(|state| state.session.clone())
            .collect()
    }

    /// 查询指定会话。
    pub async fn get_session(&self, session_id: &SessionId) -> Option<Session> {
        let sessions = self.sessions.read().await;
        sessions.get(session_id).map(|state| state.session.clone())
    }
}

#[cfg(test)]
mod tests {
    use std::path::Path;
    use std::sync::Arc;

    use super::*;
    use crate::{EventBroadcaster, OrchestratorError, SessionStatus};

    mod common {
        mod agent_orchestrator {
            pub use crate::{Executor, Result};
        }

        include!(concat!(env!("CARGO_MANIFEST_DIR"), "/tests/common/mod.rs"));
    }

    use common::MockExecutor;

    #[tokio::test]
    async fn test_create_session() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);
        let executor = MockExecutor::new("mock-executor");
        let executor_handle = executor.clone();

        let session = manager
            .create_session(Box::new(executor), Path::new("."))
            .await
            .expect("session should be created");

        assert_eq!(session.agent_name, "mock-executor");
        assert_eq!(session.status, SessionStatus::Ready);
        assert!(executor_handle.is_started());
    }

    #[tokio::test]
    async fn test_list_sessions() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);

        manager
            .create_session(Box::new(MockExecutor::new("agent-1")), Path::new("."))
            .await
            .expect("first session should be created");
        manager
            .create_session(Box::new(MockExecutor::new("agent-2")), Path::new("."))
            .await
            .expect("second session should be created");

        let sessions = manager.list_sessions().await;
        assert_eq!(sessions.len(), 2);
    }

    #[tokio::test]
    async fn test_get_session() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);

        let created = manager
            .create_session(Box::new(MockExecutor::new("agent-get")), Path::new("."))
            .await
            .expect("session should be created");

        let found = manager.get_session(&created.id).await;
        assert!(found.is_some());
        assert_eq!(found.expect("session should exist").id, created.id);

        let missing = manager.get_session(&SessionId::new()).await;
        assert!(missing.is_none());
    }

    #[tokio::test]
    async fn test_close_session() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);
        let executor = MockExecutor::new("agent-close");
        let executor_handle = executor.clone();

        let created = manager
            .create_session(Box::new(executor), Path::new("."))
            .await
            .expect("session should be created");

        manager
            .close_session(&created.id)
            .await
            .expect("session should close");

        let sessions = manager.list_sessions().await;
        assert!(sessions.is_empty());
        assert!(executor_handle.is_shutdown());
    }

    #[tokio::test]
    async fn test_send_prompt() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);

        let created = manager
            .create_session(Box::new(MockExecutor::new("agent-prompt")), Path::new("."))
            .await
            .expect("session should be created");

        manager
            .send_prompt(&created.id, "hello")
            .await
            .expect("send prompt should succeed");
    }

    #[tokio::test]
    async fn test_close_nonexistent_session() {
        let broadcaster = Arc::new(EventBroadcaster::new(16));
        let manager = SessionManager::new(broadcaster);
        let missing = SessionId::new();

        let err = manager
            .close_session(&missing)
            .await
            .expect_err("close should fail for missing session");

        match err {
            OrchestratorError::SessionNotFound(id) => assert_eq!(id, missing.to_string()),
            other => panic!("expected SessionNotFound, got: {other:?}"),
        }
    }
}
