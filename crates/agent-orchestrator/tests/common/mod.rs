use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use agent_orchestrator::{Executor, Result};
use async_trait::async_trait;

#[derive(Clone)]
pub struct MockExecutor {
    name: String,
    started: Arc<AtomicBool>,
    shutdown_called: Arc<AtomicBool>,
}

impl MockExecutor {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            started: Arc::new(AtomicBool::new(false)),
            shutdown_called: Arc::new(AtomicBool::new(false)),
        }
    }

    pub fn is_started(&self) -> bool {
        self.started.load(Ordering::SeqCst)
    }

    pub fn is_shutdown(&self) -> bool {
        self.shutdown_called.load(Ordering::SeqCst)
    }
}

#[async_trait]
impl Executor for MockExecutor {
    fn name(&self) -> &str {
        &self.name
    }

    async fn start(&mut self, _path: &Path) -> Result<()> {
        self.started.store(true, Ordering::SeqCst);
        Ok(())
    }

    async fn send_message(&mut self, _prompt: &str) -> Result<()> {
        Ok(())
    }

    async fn shutdown(&mut self) -> Result<()> {
        self.shutdown_called.store(true, Ordering::SeqCst);
        Ok(())
    }
}
