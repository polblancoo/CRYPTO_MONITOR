use teloxide::{
    macros::BotCommands,
    prelude::*,
    types::Message,
    dispatching::{HandlerExt, UpdateFilterExt},
    ApiError,
    RequestError,
    utils::command::BotCommands as TeloxideCommands,
};
use crate::db::Database;
use std::sync::Arc;
use tracing::{info, error};
use crate::auth::Auth;
use crate::models::{User, PriceAlert, AlertCondition};

#[derive(BotCommands, Clone, Debug)]
#[command(rename_rule = "lowercase", description = "Estos son los comandos disponibles:")]
pub enum Command {
    #[command(description = "muestra este mensaje")]
    Help,
    #[command(description = "inicia el bot")]
    Start,
    #[command(description = "registra tu usuario - /register <username> <password>")]
    Register { text: String },
    #[command(description = "crea una alerta de precio - /alert <symbol> <price> <above|below>")]
    Alert { text: String },
    #[command(description = "lista tus alertas activas")]
    Alerts,
    #[command(description = "elimina una alerta - /delete <id>")]
    Delete { id: i64 },
    #[command(description = "muestra los símbolos soportados")]
    Symbols,
}

impl Command {
    fn descriptions() -> String {
        <Command as TeloxideCommands>::descriptions().to_string()
    }
}

#[derive(Clone)]
pub struct TelegramBot {
    db: Arc<Database>,
}

impl TelegramBot {
    pub fn new(db: Arc<Database>) -> Self {
        Self { db }
    }

    async fn handle_command(&self, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        match cmd {
            Command::Help => {
                bot.send_message(msg.chat.id, Command::descriptions()).await?;
            }
            Command::Start => {
                bot.send_message(
                    msg.chat.id,
                    "¡Bienvenido al Bot de Alertas de Criptomonedas!\n\
                     Usa /help para ver los comandos disponibles."
                ).await?;
            }
            Command::Register { text } => {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() != 2 {
                    bot.send_message(msg.chat.id, "Uso: /register <username> <password>").await?;
                    return Ok(());
                }
                self.handle_register(bot, msg, parts[0].to_string(), parts[1].to_string()).await?;
            }
            Command::Alert { text } => {
                let parts: Vec<&str> = text.split_whitespace().collect();
                if parts.len() != 3 {
                    bot.send_message(msg.chat.id, "Uso: /alert <symbol> <price> <above|below>").await?;
                    return Ok(());
                }
                let price = match parts[1].parse::<f64>() {
                    Ok(p) => p,
                    Err(_) => {
                        bot.send_message(msg.chat.id, "El precio debe ser un número válido").await?;
                        return Ok(());
                    }
                };
                self.handle_alert(bot, msg, parts[0].to_string(), price, parts[2].to_string()).await?;
            }
            Command::Alerts => {
                self.handle_list_alerts(bot, msg).await?;
            }
            Command::Delete { id } => {
                self.handle_delete_alert(bot, msg, id).await?;
            }
            Command::Symbols => {
                self.handle_symbols(bot, msg).await?;
            }
        }
        Ok(())
    }

    async fn handle_register(&self, bot: Bot, msg: Message, username: String, password: String) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Intento de registro: username={}, chat_id={}", username, chat_id);
        
        let auth = Auth::new(self.db.as_ref());
        
        match auth.register_user(&username, &password) {
            Ok(user) => {
                if let Err(e) = self.db.update_user_telegram_chat_id(user.id, chat_id) {
                    let error_msg = format!("Error al actualizar chat_id: {}", e);
                    error!("{}", error_msg);
                    bot.send_message(msg.chat.id, "Error al vincular cuenta con Telegram").await?;
                    return Ok(());
                }

                match self.db.create_api_key(user.id) {
                    Ok(api_key) => {
                        bot.send_message(msg.chat.id, format!(
                            "✅ Registro exitoso!\n\n\
                             Tu API key es: {}\n\n\
                             Guárdala en un lugar seguro.\n\
                             Usa /help para ver los comandos disponibles.",
                            api_key.key
                        )).await?;
                    }
                    Err(e) => {
                        let error_msg = format!("Error al generar API key: {}", e);
                        error!("{}", error_msg);
                        bot.send_message(msg.chat.id, error_msg).await?;
                    }
                }
            }
            Err(e) => {
                let error_msg = format!(
                    "❌ Error al registrar usuario: {}\n\
                     El nombre de usuario ya existe o es inválido.\n\
                     Intenta con otro nombre de usuario.",
                    e
                );
                error!("{}", error_msg);
                bot.send_message(msg.chat.id, error_msg).await?;
            }
        }
        
        Ok(())
    }

    async fn handle_alert(&self, bot: Bot, msg: Message, symbol: String, price: f64, condition: String) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Nueva alerta: symbol={}, price={}, condition={}, chat_id={}", 
              symbol, price, condition, chat_id);

        // Obtener usuario por chat_id
        let user = match self.get_user_by_chat_id(chat_id).await? {
            Some(user) => user,
            None => {
                bot.send_message(msg.chat.id, "❌ Debes registrarte primero usando /register").await?;
                return Ok(());
            }
        };

        // Validar condición
        let alert_condition = match condition.to_lowercase().as_str() {
            "above" => AlertCondition::Above,
            "below" => AlertCondition::Below,
            _ => {
                bot.send_message(msg.chat.id, "❌ Condición inválida. Usa 'above' o 'below'").await?;
                return Ok(());
            }
        };

        // Crear alerta
        let alert = PriceAlert {
            id: None,
            user_id: user.id,
            symbol: symbol.to_uppercase(),
            target_price: price,
            condition: alert_condition,
            created_at: chrono::Utc::now().timestamp(),
            triggered_at: None,
            is_active: true,
        };

        match self.db.save_alert(&alert) {
            Ok(_) => {
                bot.send_message(msg.chat.id, format!(
                    "✅ Alerta creada exitosamente!\n\n\
                     Símbolo: {}\n\
                     Precio objetivo: ${:.2}\n\
                     Condición: {:?}",
                    alert.symbol, alert.target_price, alert.condition
                )).await?;
            }
            Err(e) => {
                error!("Error al crear alerta: {}", e);
                bot.send_message(msg.chat.id, "❌ Error al crear la alerta").await?;
            }
        }

        Ok(())
    }

    // Método auxiliar para obtener usuario por chat_id
    async fn get_user_by_chat_id(&self, chat_id: i64) -> ResponseResult<Option<User>> {
        self.db.get_user_by_telegram_id(chat_id)
            .map_err(|e| RequestError::Api(ApiError::Unknown(e.to_string())))
    }

    async fn handle_list_alerts(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Listando alertas para chat_id={}", chat_id);

        // Obtener usuario por chat_id
        let user = match self.get_user_by_chat_id(chat_id).await? {
            Some(user) => user,
            None => {
                bot.send_message(msg.chat.id, "❌ Debes registrarte primero usando /register").await?;
                return Ok(());
            }
        };

        // Obtener alertas del usuario
        match self.db.get_user_alerts(user.id) {
            Ok(alerts) => {
                if alerts.is_empty() {
                    bot.send_message(msg.chat.id, "No tienes alertas configuradas").await?;
                    return Ok(());
                }

                let mut response = String::from("📊 Tus alertas:\n\n");
                for alert in alerts {
                    let status = if alert.is_active { "🟢 Activa" } else { "🔴 Disparada" };
                    response.push_str(&format!(
                        "ID: {}\n\
                         Símbolo: {}\n\
                         Precio: ${:.2}\n\
                         Condición: {:?}\n\
                         Estado: {}\n\n",
                        alert.id.unwrap_or(-1),
                        alert.symbol,
                        alert.target_price,
                        alert.condition,
                        status
                    ));
                }
                bot.send_message(msg.chat.id, response).await?;
            }
            Err(e) => {
                error!("Error al obtener alertas: {}", e);
                bot.send_message(msg.chat.id, "❌ Error al obtener las alertas").await?;
            }
        }

        Ok(())
    }

    async fn handle_delete_alert(&self, bot: Bot, msg: Message, alert_id: i64) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Eliminando alerta {} para chat_id={}", alert_id, chat_id);

        // Obtener usuario
        let user = match self.get_user_by_chat_id(chat_id).await? {
            Some(user) => user,
            None => {
                bot.send_message(msg.chat.id, "❌ Debes registrarte primero usando /register").await?;
                return Ok(());
            }
        };

        // Verificar que la alerta existe y pertenece al usuario
        match self.db.get_alert(alert_id) {
            Ok(Some(alert)) => {
                if alert.user_id != user.id {
                    bot.send_message(msg.chat.id, "❌ Esta alerta no te pertenece").await?;
                    return Ok(());
                }

                match self.db.delete_alert(alert_id) {
                    Ok(_) => {
                        bot.send_message(msg.chat.id, format!(
                            "✅ Alerta eliminada exitosamente!\n\
                             ID: {}\n\
                             Símbolo: {}\n\
                             Precio: ${:.2}",
                            alert_id, alert.symbol, alert.target_price
                        )).await?;
                    }
                    Err(e) => {
                        error!("Error al eliminar alerta: {}", e);
                        bot.send_message(msg.chat.id, "❌ Error al eliminar la alerta").await?;
                    }
                }
            }
            Ok(None) => {
                bot.send_message(msg.chat.id, "❌ No se encontró la alerta especificada").await?;
            }
            Err(e) => {
                error!("Error al buscar alerta: {}", e);
                bot.send_message(msg.chat.id, "❌ Error al buscar la alerta").await?;
            }
        }

        Ok(())
    }

    async fn handle_symbols(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        info!("Mostrando símbolos soportados");
        
        let symbols = vec![
            "BTC (Bitcoin)",
            "ETH (Ethereum)",
            "USDT (Tether)",
            "BNB (Binance Coin)",
            "SOL (Solana)",
            "XRP (Ripple)",
            "USDC (USD Coin)",
            "ADA (Cardano)",
            "AVAX (Avalanche)",
            "DOGE (Dogecoin)",
        ];

        let response = format!(
            "🪙 Símbolos soportados:\n\n{}\n\n\
             Uso: /alert <symbol> <price> <above|below>\n\
             Ejemplo: /alert BTC 42000 above",
            symbols.join("\n")
        );

        bot.send_message(msg.chat.id, response).await?;
        Ok(())
    }

    pub async fn run(self) {
        let bot = Bot::new(std::env::var("TELEGRAM_BOT_TOKEN").expect("TELEGRAM_BOT_TOKEN must be set"));
        
        info!("Starting Telegram bot...");
        
        let handler = Update::filter_message()
            .filter_command::<Command>()
            .endpoint(move |bot: Bot, msg: Message, cmd: Command| {
                let bot_handler = self.clone();
                async move {
                    if let Err(e) = bot_handler.handle_command(bot, msg, cmd).await {
                        error!("Error handling command: {}", e);
                        return Err(RequestError::Api(ApiError::Unknown(e.to_string())));
                    }
                    Ok(())
                }
            });

        Dispatcher::builder(bot, handler)
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }
} 