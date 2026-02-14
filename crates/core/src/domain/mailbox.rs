use super::{DomainError, MailId, UserId};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum MailCategory {
    System,
    SubmissionResult,
    Contest,
    Security,
    Activity,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum MailStatus {
    #[default]
    Unread,
    Read,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MailTitle(String);

impl MailTitle {
    pub const MAX_LEN: usize = 120;

    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyMailTitle);
        }

        let len = trimmed.chars().count();
        if len > Self::MAX_LEN {
            return Err(DomainError::InvalidMailTitleLength(len));
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct MailContent(String);

impl MailContent {
    pub const MAX_LEN: usize = 10_000;

    pub fn new(value: impl Into<String>) -> Result<Self, DomainError> {
        let value = value.into();
        let trimmed = value.trim();

        if trimmed.is_empty() {
            return Err(DomainError::EmptyMailContent);
        }

        let len = trimmed.chars().count();
        if len > Self::MAX_LEN {
            return Err(DomainError::InvalidMailContentLength(len));
        }

        Ok(Self(trimmed.to_string()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MailMessage {
    id: MailId,
    recipient_id: UserId,
    category: MailCategory,
    status: MailStatus,
    title: MailTitle,
    content: MailContent,
}

impl MailMessage {
    pub fn new(
        recipient_id: UserId,
        category: MailCategory,
        title: MailTitle,
        content: MailContent,
    ) -> Self {
        Self {
            id: MailId::new(),
            recipient_id,
            category,
            status: MailStatus::Unread,
            title,
            content,
        }
    }

    pub fn id(&self) -> MailId {
        self.id
    }

    pub fn recipient_id(&self) -> UserId {
        self.recipient_id
    }

    pub fn category(&self) -> MailCategory {
        self.category
    }

    pub fn status(&self) -> MailStatus {
        self.status
    }

    pub fn title(&self) -> &MailTitle {
        &self.title
    }

    pub fn content(&self) -> &MailContent {
        &self.content
    }

    pub fn mark_read(&mut self) {
        self.status = MailStatus::Read;
    }

    pub fn mark_unread(&mut self) {
        self.status = MailStatus::Unread;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_mail_title_is_created() {
        let title = MailTitle::new("  系统通知  ").expect("title should be valid");
        assert_eq!(title.as_str(), "系统通知");
    }

    #[test]
    fn empty_mail_title_is_rejected() {
        let err = MailTitle::new("   ").expect_err("empty title should be rejected");
        assert_eq!(err, DomainError::EmptyMailTitle);
    }

    #[test]
    fn too_long_mail_title_is_rejected() {
        let long = "a".repeat(MailTitle::MAX_LEN + 1);
        let err = MailTitle::new(long).expect_err("too long title should be rejected");
        assert_eq!(err, DomainError::InvalidMailTitleLength(121));
    }

    #[test]
    fn valid_mail_content_is_created() {
        let content = MailContent::new("  评测完成，请查看结果。 ").expect("content should be valid");
        assert_eq!(content.as_str(), "评测完成，请查看结果。");
    }

    #[test]
    fn too_long_mail_content_is_rejected() {
        let long = "a".repeat(MailContent::MAX_LEN + 1);
        let err = MailContent::new(long).expect_err("too long content should be rejected");
        assert_eq!(err, DomainError::InvalidMailContentLength(10_001));
    }

    #[test]
    fn mail_message_mark_read_and_unread() {
        let recipient_id = UserId::new();
        let title = MailTitle::new("提交结果通知").expect("title should be valid");
        let content = MailContent::new("你的提交已通过全部测试。").expect("content should be valid");
        let mut mail = MailMessage::new(
            recipient_id,
            MailCategory::SubmissionResult,
            title,
            content,
        );

        assert_eq!(mail.status(), MailStatus::Unread);
        mail.mark_read();
        assert_eq!(mail.status(), MailStatus::Read);
        mail.mark_unread();
        assert_eq!(mail.status(), MailStatus::Unread);
    }
}
