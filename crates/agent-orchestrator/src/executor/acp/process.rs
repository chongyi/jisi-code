use std::path::Path;
use std::process::Stdio;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{Child, ChildStdin, ChildStdout, Command};
use tracing::{debug, info, warn};

use crate::error::{OrchestratorError, Result};

pub struct AcpProcess {
    child: Child,
    stdin: ChildStdin,
    stdout: BufReader<ChildStdout>,
}

impl AcpProcess {
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

    pub async fn send_line(&mut self, line: &str) -> Result<()> {
        debug!(line = line, "sending line to ACP process");
        self.stdin.write_all(line.as_bytes()).await?;
        self.stdin.write_all(b"\n").await?;
        self.stdin.flush().await?;
        Ok(())
    }

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

    pub async fn kill(&mut self) -> Result<()> {
        info!("killing ACP process");
        if let Err(err) = self.child.kill().await {
            warn!(error = %err, "failed to kill ACP process");
            return Err(err.into());
        }
        info!("ACP process killed");
        Ok(())
    }
}
