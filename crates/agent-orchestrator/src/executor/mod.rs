//! 执行器抽象层。
//!
//! 该模块定义了统一的 `Executor` 接口，用于屏蔽不同 AI 编码工具
//! （如 Claude Code、Codex、OpenCode）在启动、消息发送与关闭流程上的差异。

use std::path::Path;

use async_trait::async_trait;

use crate::error::Result;

pub mod acp;

/// 执行器抽象接口。
///
/// 不同执行器实现通过该 trait 暴露统一生命周期操作：
/// 1. 启动执行器；
/// 2. 发送任务消息；
/// 3. 关闭执行器并释放资源。
#[async_trait]
pub trait Executor: Send + Sync {
    /// 返回执行器名称。
    ///
    /// 该名称用于日志、诊断和执行器类型识别。
    fn name(&self) -> &str;

    /// 启动执行器。
    ///
    /// `project_path` 为目标项目根目录，执行器应在该目录上下文中初始化。
    async fn start(&mut self, project_path: &Path) -> Result<()>;

    /// 向执行器发送消息。
    ///
    /// `prompt` 通常为用户需求或编排器生成的执行指令。
    async fn send_message(&mut self, prompt: &str) -> Result<()>;

    /// 关闭执行器。
    ///
    /// 实现应尽量保证幂等，确保重复调用不会导致未定义行为。
    async fn shutdown(&mut self) -> Result<()>;
}
