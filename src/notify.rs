use std::error::Error;
use teloxide::{prelude::*, types::ChatId, Bot};
use tracing::{info, error, debug};
use tokio::time::{sleep, Duration};

pub struct NotificationService {
    bot: Bot,
}

impl NotificationService {
    pub fn new(token: String) -> Self {
        info!("Inicializando NotificationService con token: {}...", &token[..8]);
        Self {
            bot: Bot::new(token),
        }
    }

    pub async fn send_alert(&self, user_id: i64, message: &str) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Preparando envío de notificación");
        debug!("Usuario ID: {}", user_id);
        debug!("Mensaje: {}", message);
        
        // Verificar que el user_id es válido
        if user_id <= 0 {
            error!("ID de usuario inválido: {}", user_id);
            return Err("ID de usuario inválido".into());
        }

        // Convertir user_id a ChatId
        let chat_id = ChatId(user_id);
        info!("Intentando enviar mensaje a chat_id: {}", user_id);

        // Escapar caracteres especiales para MarkdownV2
        let formatted_message = escape_markdown(message);

        // Intentar enviar el mensaje
        match self.bot.send_message(chat_id, formatted_message)
            .parse_mode(teloxide::types::ParseMode::MarkdownV2)
            .await {
                Ok(_) => Ok(()),
                Err(e) => Err(Box::new(e) as Box<dyn Error + Send + Sync>)
            }
    }

    pub async fn verify_bot(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        const MAX_RETRIES: u32 = 3;
        const INITIAL_RETRY_DELAY: u64 = 5;
        const MAX_RETRY_DELAY: u64 = 30;

        let mut retry_count = 0;
        loop {
            match self.bot.get_me().await {
                Ok(me) => {
                    info!("Bot verificado: @{}", me.username());
                    return Ok(());
                }
                Err(e) => {
                    retry_count += 1;
                    if retry_count >= MAX_RETRIES {
                        error!("Error al verificar bot después de {} intentos: {}", MAX_RETRIES, e);
                        return Err(Box::new(e));
                    }

                    let delay = (INITIAL_RETRY_DELAY * 2u64.pow(retry_count - 1))
                        .min(MAX_RETRY_DELAY);
                    
                    error!(
                        "Error al verificar bot (intento {}/{}): {}. Reintentando en {} segundos...",
                        retry_count, MAX_RETRIES, e, delay
                    );
                    sleep(Duration::from_secs(delay)).await;
                }
            }
        }
    }
}

fn escape_markdown(text: &str) -> String {
    text.replace(".", "\\.")
        .replace("-", "\\-")
        .replace("(", "\\(")
        .replace(")", "\\)")
        .replace("!", "\\!")
        .replace(">", "\\>")
        .replace("<", "\\<")
        .replace("+", "\\+")
        .replace("$", "\\$")
        .replace("%", "\\%")
        .replace("#", "\\#")
        .replace("{", "\\{")
        .replace("}", "\\}")
        .replace("=", "\\=")
        .replace("|", "\\|")
        .replace("~", "\\~")
        .replace("`", "\\`")
        .replace("*", "\\*")
        .replace("[", "\\[")
        .replace("]", "\\]")
        .replace("_", "\\_")
} 