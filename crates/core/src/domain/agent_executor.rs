use async_trait::async_trait;
use thiserror::Error;

use super::{Language, ProblemId, Score, SubmissionId, SubmissionStatus, UserId};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionRequest {
    pub submission_id: SubmissionId,
    pub user_id: UserId,
    pub problem_id: ProblemId,
    pub language: Language,
    pub source_code: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AgentExecutionResult {
    pub status: SubmissionStatus,
    pub score: Score,
    pub compiler_output: Option<String>,
    pub execution_output: Option<String>,
    pub runtime_ms: Option<u32>,
    pub memory_kb: Option<u32>,
}

impl AgentExecutionResult {
    pub fn accepted(score: Score, runtime_ms: u32, memory_kb: u32) -> Self {
        Self {
            status: SubmissionStatus::Accepted,
            score,
            compiler_output: None,
            execution_output: None,
            runtime_ms: Some(runtime_ms),
            memory_kb: Some(memory_kb),
        }
    }

    pub fn failed(
        status: SubmissionStatus,
        compiler_output: Option<String>,
        execution_output: Option<String>,
    ) -> Self {
        Self {
            status,
            score: Score::default(),
            compiler_output,
            execution_output,
            runtime_ms: None,
            memory_kb: None,
        }
    }
}

#[derive(Debug, Clone, Error, PartialEq, Eq)]
pub enum AgentExecutorError {
    #[error("agent executor unavailable: {0}")]
    Unavailable(String),
    #[error("agent executor timeout")]
    Timeout,
    #[error("agent executor failed: {0}")]
    Failed(String),
}

#[async_trait]
pub trait AgentExecutor: Send + Sync {
    async fn execute(
        &self,
        request: AgentExecutionRequest,
    ) -> Result<AgentExecutionResult, AgentExecutorError>;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn accepted_result_has_expected_fields() {
        let score = Score::new(100).expect("score should be valid");
        let result = AgentExecutionResult::accepted(score, 12, 1024);

        assert_eq!(result.status, SubmissionStatus::Accepted);
        assert_eq!(result.score, score);
        assert_eq!(result.runtime_ms, Some(12));
        assert_eq!(result.memory_kb, Some(1024));
        assert!(result.compiler_output.is_none());
    }

    #[test]
    fn failed_result_defaults_score_and_resource_usage() {
        let result = AgentExecutionResult::failed(
            SubmissionStatus::CompileError,
            Some("compile failed".to_string()),
            None,
        );

        assert_eq!(result.status, SubmissionStatus::CompileError);
        assert_eq!(result.score, Score::default());
        assert_eq!(result.runtime_ms, None);
        assert_eq!(result.memory_kb, None);
        assert_eq!(result.compiler_output.as_deref(), Some("compile failed"));
    }
}
