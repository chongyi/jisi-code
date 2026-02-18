//! 统一的应用状态。

use std::sync::Arc;

use agent_orchestrator::Orchestrator;
use system_capabilities::FileSystemCapabilities;

/// 统一的应用状态，包含所有服务共享的数据。
#[derive(Clone)]
pub struct AppState {
    /// Agent 编排器。
    pub orchestrator: Arc<Orchestrator>,
    /// 文件系统能力。
    pub filesystem: FileSystemCapabilities,
}

impl AppState {
    /// 创建新的应用状态。
    pub fn new(orchestrator: Arc<Orchestrator>) -> Self {
        Self {
            orchestrator,
            filesystem: FileSystemCapabilities::new(),
        }
    }
}
