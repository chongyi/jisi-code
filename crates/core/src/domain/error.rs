use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid score: {0}. score must be in [0, 100]")]
    InvalidScore(u16),
}
