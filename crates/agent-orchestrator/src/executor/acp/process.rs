use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, info, warn};

use crate::error::{OrchestratorError, Result};

/// ACP 子进程句柄与标准输入输出通道封装。
pub struct AcpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl AcpProcess {
    /// 启动 ACP 子进程。
    ///
    /// `command` 和 `args` 用于构造命令行，`project_path` 作为工作目录。
    pub async fn spawn(
        command: &str,
        args: &[String],
        project_path: &Path,
        env_vars: &[(String, String)],
    ) -> Result<Self> {
        info!(
            command = command,
            args = ?args,
            project_path = %project_path.display(),
            "spawning ACP process"
        );

        let mut cmd = Command::new(command);
        cmd.args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .kill_on_drop(true)
            .current_dir(project_path);

        for (key, value) in env_vars {
            cmd.env(key, value);
        }

        let mut child = cmd.spawn()?;
        let stdin = child.stdin.take().ok_or_else(|| {
            OrchestratorError::Executor("failed to capture ACP process stdin".to_string())
        })?;
        let stdout = child.stdout.take().ok_or_else(|| {
            OrchestratorError::Executor("failed to capture ACP process stdout".to_string())
        })?;

        info!("ACP process spawned successfully");

        Ok(Self {
            child,
            stdin,
            stdout: BufReader::new(stdout),
        })
    }

    /// 向 ACP 进程写入一行 JSON-RPC 消息。
    pub async fn send_line(&mut self, line: &str) -> Result<()> {
        debug!(line = line, "sending line to ACP process");
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

    /// 从 ACP 进程读取一行输出。
    ///
    /// 当子进程输出 EOF 时返回 `Ok(None)`。
    pub async fn read_line(&mut self) -> Result<Option<String>> {
        let mut line = String::new();
        let bytes = self.stdout.read_line(&mut line).await?;

        if bytes == 0 {
            warn!("ACP process stdout reached EOF");
            return Ok(None);
        }

        let trimmed = line.trim_end().to_string();
        debug!(line = trimmed, "received line from ACP process");
        Ok(Some(trimmed))
    }

    /// 终止 ACP 子进程。
    pub async fn kill(&mut self) -> Result<()> {
        info!("killing ACP process");
        if let Err(err) = self.child.kill().await {
            warn!(error = %err, "failed to kill ACP process");
            return Err(err.into());
        }
        info!("ACP process killed");
        Ok(())
    }

    /// Non-blockingly checks whether the process has exited.
    /// Returns Some(ExitStatus) if exited, None if still running.
    pub fn try_wait(&mut self) -> Result<Option<std::process::ExitStatus>> {
        match self.child.try_wait()? {
            Some(status) => Ok(Some(status)),
            None => Ok(None),
        }
    }
}
