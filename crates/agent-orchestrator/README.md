# agent-orchestrator

`agent-orchestrator` 是一个面向 AI 编码工具的统一编排模块，提供一致的会话管理、消息转发与事件分发能力。该 crate 目标是将不同 Agent（当前已支持 ACP/Claude Code）收敛到同一套异步接口，降低上层集成复杂度。

## 功能特性

- 统一的 Agent 管理接口：通过 `Orchestrator` 对外暴露一致的会话与消息 API。
- ACP 协议支持（Claude Code）：内置 `AcpExecutor`，可通过 stdio 与 Claude Code Agent 通信。
- 基于配置文件的 Agent 管理：使用 `agents.toml` 声明 Agent 列表、命令参数、环境变量与启用状态。
- 事件驱动的异步架构：基于广播事件流，支持多订阅者消费会话事件。
- 会话生命周期管理：覆盖会话创建、消息发送、错误上报与关闭。
- 计划中：WebSocket API、Codex/OpenCode 执行器支持。

## 快速开始

### 1) 配置 `agents.toml`

以下示例与项目根目录当前配置保持一致（`claude` 参数为 `agent --transport stdio`）：

```toml
event_buffer_size = 1000

[[agents]]
id = "claude-code-acp"
display_name = "Claude Code (ACP)"
type = "acp"
command = "claude"
args = ["agent", "--transport", "stdio"]
enabled = true

[[agents.env]]
key = "CLAUDE_CODE_ENTRYPOINT"
value = "agent-orchestrator"
```

### 2) 使用示例

```rust
use std::path::Path;

use agent_orchestrator::{Orchestrator, OrchestratorConfig, OrchestratorEvent, Result};

#[tokio::main]
async fn main() -> Result<()> {
    // 读取编排器配置
    let config = OrchestratorConfig::from_file("agents.toml")?;

    // 初始化编排器
    let orchestrator = Orchestrator::new(config)?;

    // 订阅事件流（可在发送消息前先订阅，避免漏事件）
    let mut events = orchestrator.subscribe_events();

    // 创建会话
    let session = orchestrator
        .create_session("claude-code-acp", Path::new("."))
        .await?;

    // 发送提示词
    orchestrator
        .send_prompt(session.id(), "请分析当前仓库并给出重构建议")
        .await?;

    // 消费部分事件（示例中简单读取若干条）
    for _ in 0..10 {
        match events.recv().await {
            Ok(OrchestratorEvent::SessionCreated {
                session_id,
                agent_name,
            }) => {
                println!("session created: {} ({})", session_id, agent_name);
            }
            Ok(OrchestratorEvent::ContentDelta { session_id, content }) => {
                println!("[{}] delta: {}", session_id, content);
            }
            Ok(OrchestratorEvent::ToolCall {
                session_id,
                tool_name,
                args,
            }) => {
                println!("[{}] tool call: {} args={}", session_id, tool_name, args);
            }
            Ok(OrchestratorEvent::SessionError { session_id, error }) => {
                eprintln!("[{}] error: {}", session_id, error);
            }
            Ok(OrchestratorEvent::SessionClosed { session_id }) => {
                println!("session closed: {}", session_id);
                break;
            }
            Err(err) => {
                eprintln!("event stream closed: {}", err);
                break;
            }
        }
    }

    // 关闭会话
    orchestrator.close_session(session.id()).await?;

    Ok(())
}
```

## 架构设计

`agent-orchestrator` 采用分层编排：上层负责统一 API 与配置驱动，中层负责会话状态与生命周期，下层通过执行器抽象对接具体 Agent 协议。

```text
+------------------------------------------------------+
|                   Orchestrator                       |
|  - new() / create_session() / send_prompt() ...      |
+--------------------------+---------------------------+
                           |
                           v
+------------------------------------------------------+
|                  SessionManager                       |
|  - Session 生命周期管理                               |
|  - 会话路由与状态维护                                 |
+--------------------------+---------------------------+
                           |
                           v
+------------------------------------------------------+
|                   Executor Trait                      |
|  - start(project_path)                                |
|  - send_message(prompt)                               |
|  - shutdown()                                         |
+---------------------+----------------+---------------+
                      |                |
                      v                v
          +------------------+   +------------------+
          |   AcpExecutor    |   |  CodexExecutor   | (计划)
          | (Claude Code)    |   |  OpenCodeExec.   | (计划)
          +------------------+   +------------------+

+------------------------------------------------------+
|               EventBroadcaster / EventStream          |
|  - SessionCreated / ContentDelta / ToolCall ...       |
+------------------------------------------------------+
```

## API 概览

### `Orchestrator`

- `new(config: OrchestratorConfig) -> Result<Orchestrator>`
  - 初始化编排器、事件广播器与会话管理器。
- `create_session(agent_id: &str, project_path: &Path) -> Result<Session>`
  - 根据 `agent_id` 选择已启用 Agent，创建执行器并启动会话。
- `send_prompt(session_id: &SessionId, prompt: &str) -> Result<()>`
  - 向指定会话发送提示词。
- `close_session(session_id: &SessionId) -> Result<()>`
  - 关闭会话并释放执行器资源。
- `subscribe_events() -> EventStream`
  - 订阅事件流。
- `available_agents() -> Vec<AgentInfo>`
  - 返回已启用 Agent 列表。

### 事件类型

- `SessionCreated { session_id, agent_name }`
- `ContentDelta { session_id, content }`
- `ToolCall { session_id, tool_name, args }`
- `SessionError { session_id, error }`
- `SessionClosed { session_id }`

### 配置结构

- `OrchestratorConfig`
  - `agents: Vec<AgentConfig>`
  - `event_buffer_size: usize`（默认 `1000`）
- `AgentConfig`
  - `id`, `display_name`, `type`, `command`, `args`, `env`, `enabled`
- `AgentType`
  - `acp`（已支持）
  - `codex`（预留）
  - `opencode`（预留）

## 开发指南

### 构建与检查

在仓库根目录执行：

```bash
cargo build -p agent-orchestrator
cargo test -p agent-orchestrator
cargo clippy -p agent-orchestrator --all-targets -- -D warnings
```

### 如何添加新的 `Executor` 实现

1. 在 `src/executor/` 下新增模块（例如 `codex/`），实现 `Executor` trait：
   - `start(&mut self, project_path: &Path)`
   - `send_message(&mut self, prompt: &str)`
   - `shutdown(&mut self)`
2. 在 `src/executor/mod.rs` 中导出新执行器。
3. 在 `src/orchestrator.rs` 的 `create_session` 分支中，根据 `AgentType` 实例化新执行器。
4. 在 `src/config.rs` 的配置解析与示例中补充对应配置。
5. 为新执行器补充单元测试/集成测试，确保会话创建、消息发送、关闭流程与事件上报可用。

## 项目状态

- Phase 1：已完成
  - 基础编排器 API
  - 会话管理
  - ACP（Claude Code）执行器
  - 事件系统
- Phase 2：计划中
  - WebSocket API
  - Codex/OpenCode 执行器
  - 更完整的可观测性与错误恢复能力
