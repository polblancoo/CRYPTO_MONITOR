use teloxide::prelude::*;

pub struct NotificationService {
    bot: Bot,
}

impl NotificationService {
    pub fn new(token: String) -> Self {
        Self {
            bot: Bot::new(token),
        }
    }

    pub async fn send_alert(&self, user_id: i64, message: &str) -> Result<(), teloxide::RequestError> {
        self.bot.send_message(ChatId(user_id), message).await?;
        Ok(())
    }
} 