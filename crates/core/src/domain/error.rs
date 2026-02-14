use thiserror::Error;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum DomainError {
    #[error("invalid score: {0}. score must be in [0, 100]")]
    InvalidScore(u16),
    #[error("invalid mail title: title cannot be empty")]
    EmptyMailTitle,
    #[error(
        "invalid mail title length: {0}. title length must be in [1, {max}]",
        max = crate::domain::mailbox::MailTitle::MAX_LEN
    )]
    InvalidMailTitleLength(usize),
    #[error("invalid mail content: content cannot be empty")]
    EmptyMailContent,
    #[error(
        "invalid mail content length: {0}. content length must be in [1, {max}]",
        max = crate::domain::mailbox::MailContent::MAX_LEN
    )]
    InvalidMailContentLength(usize),
}
