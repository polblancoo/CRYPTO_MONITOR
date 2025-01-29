use std::error::Error;
use teloxide::{prelude::*, types::ChatId};
use tracing::{info, error, debug};

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

        // Intentar enviar el mensaje
        match self.bot.send_message(chat_id, message).await {
            Ok(message) => {
                info!("Notificación enviada exitosamente");
                debug!("Message ID: {}", message.id);
                Ok(())
            }
            Err(e) => {
                error!("Error detallado al enviar notificación:");
                error!("  Tipo de error: {:?}", e);
                error!("  Descripción: {}", e);
                error!("  Chat ID: {}", user_id);
                Err(Box::new(e))
            }
        }
    }

    // Método para verificar el estado del bot
    pub async fn verify_bot(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Verificando estado del bot");
        match self.bot.get_me().await {
            Ok(me) => {
                info!("Bot verificado: @{}", me.username());
                Ok(())
            }
            Err(e) => {
                error!("Error al verificar bot: {}", e);
                Err(Box::new(e))
            }
        }
    }
} 