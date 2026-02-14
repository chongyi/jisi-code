use std::collections::{HashMap, HashSet};

use super::{MailCategory, MailContent, MailMessage, MailTitle, UserId};

#[derive(Debug, Clone, Default)]
pub struct MailRouter {
    subscribers: HashMap<MailCategory, HashSet<UserId>>,
}

impl MailRouter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn subscribe(&mut self, category: MailCategory, user_id: UserId) -> bool {
        self.subscribers
            .entry(category)
            .or_default()
            .insert(user_id)
    }

    pub fn unsubscribe(&mut self, category: MailCategory, user_id: UserId) -> bool {
        match self.subscribers.get_mut(&category) {
            Some(users) => users.remove(&user_id),
            None => false,
        }
    }

    pub fn route(&self, category: MailCategory, primary_recipient: Option<UserId>) -> Vec<UserId> {
        let mut recipients = HashSet::new();

        if let Some(user_id) = primary_recipient {
            recipients.insert(user_id);
        }

        if let Some(subscribers) = self.subscribers.get(&category) {
            recipients.extend(subscribers.iter().copied());
        }

        let mut routed: Vec<UserId> = recipients.into_iter().collect();
        routed.sort_by_key(|id| id.into_inner());
        routed
    }

    pub fn dispatch(
        &self,
        category: MailCategory,
        title: MailTitle,
        content: MailContent,
        primary_recipient: Option<UserId>,
    ) -> Vec<MailMessage> {
        self.route(category, primary_recipient)
            .into_iter()
            .map(|recipient_id| {
                MailMessage::new(recipient_id, category, title.clone(), content.clone())
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::MailStatus;

    fn user_ids() -> (UserId, UserId, UserId) {
        (UserId::new(), UserId::new(), UserId::new())
    }

    #[test]
    fn route_returns_primary_recipient_when_no_subscribers() {
        let router = MailRouter::new();
        let (u1, _, _) = user_ids();

        let recipients = router.route(MailCategory::Security, Some(u1));

        assert_eq!(recipients, vec![u1]);
    }

    #[test]
    fn route_merges_primary_and_subscribers_without_duplicates() {
        let mut router = MailRouter::new();
        let (u1, u2, _) = user_ids();

        router.subscribe(MailCategory::System, u1);
        router.subscribe(MailCategory::System, u2);

        let recipients = router.route(MailCategory::System, Some(u1));

        assert_eq!(recipients.len(), 2);
        assert!(recipients.contains(&u1));
        assert!(recipients.contains(&u2));
    }

    #[test]
    fn unsubscribe_removes_subscriber() {
        let mut router = MailRouter::new();
        let (u1, _, _) = user_ids();

        assert!(router.subscribe(MailCategory::Contest, u1));
        assert!(router.unsubscribe(MailCategory::Contest, u1));

        let recipients = router.route(MailCategory::Contest, None);
        assert!(recipients.is_empty());
    }

    #[test]
    fn dispatch_creates_mail_for_all_routed_recipients() {
        let mut router = MailRouter::new();
        let (u1, u2, _) = user_ids();
        router.subscribe(MailCategory::Activity, u2);

        let title = MailTitle::new("活动通知").expect("valid title");
        let content = MailContent::new("你关注的题目有新动态").expect("valid content");

        let mails = router.dispatch(MailCategory::Activity, title, content, Some(u1));

        assert_eq!(mails.len(), 2);

        let recipients: Vec<UserId> = mails.iter().map(|mail| mail.recipient_id()).collect();
        assert!(recipients.contains(&u1));
        assert!(recipients.contains(&u2));

        for mail in mails {
            assert_eq!(mail.category(), MailCategory::Activity);
            assert_eq!(mail.status(), MailStatus::Unread);
            assert_eq!(mail.title().as_str(), "活动通知");
            assert_eq!(mail.content().as_str(), "你关注的题目有新动态");
        }
    }
}
