use std::sync::Arc;
use teloxide::{
    macros::BotCommands,
    prelude::*,
    types::{
        Message, 
        InlineKeyboardMarkup, 
        InlineKeyboardButton,
        ParseMode,
        CallbackQuery,
    },
    dispatching::{HandlerExt, UpdateFilterExt},
    ApiError,
    RequestError,
    utils::command::BotCommands as TeloxideCommands,
};
use crate::db::Database;
use tracing::{info, error};
use crate::auth::Auth;
use crate::models::{User, PriceAlert, AlertCondition, AlertType, UserState, PriceAlertStep, DepegAlertStep, PairAlertStep};
use crate::config::CONFIG;
use crate::exchanges::ExchangeManager;
use tokio_rusqlite::Error as AsyncSqliteError;

mod handlers;

#[derive(BotCommands, Clone, Debug)]
#[command(
    rename_rule = "lowercase",
    description = "Estos son los comandos disponibles:"
)]
pub enum Command {
    #[command(description = "muestra este mensaje")]
    Help,
    #[command(description = "inicia el bot")]
    Start,
    #[command(description = "registra tu usuario - /register <username> <password>")]
    Register { text: String },
    #[command(description = "crea una alerta de precio")]
    Alert,
    #[command(description = "crea alerta de depeg")]
    Depeg,
    #[command(description = "crea alerta de par")]
    PairDepeg,
    #[command(description = "lista tus alertas activas")]
    Alerts,
    #[command(description = "elimina una alerta")]
    Delete,
    #[command(description = "muestra los símbolos soportados")]
    Symbols,
    #[command(description = "Ver balance")]
    Balance,
    #[command(description = "Crear orden")]
    Order(String),
    #[command(description = "Ver órdenes abiertas")]
    Orders,
}

impl Command {
    fn descriptions() -> String {
        <Command as TeloxideCommands>::descriptions().to_string()
    }

    fn from_str(s: &str) -> Result<Self, &'static str> {
        let lowercase = s.to_lowercase();
        match lowercase.as_str() {
            "/help" | "help" => Ok(Command::Help),
            "/start" | "start" => Ok(Command::Start),
            // ... otros casos
            _ => Err("Comando no reconocido")
        }
    }
}

#[derive(Debug)]
pub enum BotError {
    RequestError(RequestError),
    InvalidInput(String),
    DatabaseError(AsyncSqliteError),
}

impl From<RequestError> for BotError {
    fn from(err: RequestError) -> Self {
        BotError::RequestError(err)
    }
}

impl From<AsyncSqliteError> for BotError {
    fn from(err: AsyncSqliteError) -> Self {
        BotError::DatabaseError(err)
    }
}

impl From<std::num::ParseFloatError> for BotError {
    fn from(err: std::num::ParseFloatError) -> Self {
        BotError::InvalidInput(format!("Número inválido: {}", err))
    }
}

#[derive(Clone)]
pub struct TelegramBot {
    pub db: Arc<Database>,
    pub exchange_manager: Arc<ExchangeManager>,
    pub bot: Bot,
}

impl TelegramBot {
    pub fn new(db: Arc<Database>, exchange_manager: Arc<ExchangeManager>, bot: Bot) -> Self {
        Self { 
            db,
            exchange_manager,
            bot,
        }
    }

    // Helper para convertir errores de SQLite a RequestError
    fn db_error_to_request_error(e: tokio_rusqlite::Error) -> RequestError {
        RequestError::Api(ApiError::Unknown(e.to_string()))
    }

    async fn handle_command(&self, bot: Bot, msg: Message, cmd: Command) -> ResponseResult<()> {
        info!("Manejando comando: {:?}", cmd);
        
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
            Command::Alert => {
                self.handle_alert_creation(bot, msg).await?;
            }
            Command::Depeg => {
                self.handle_depeg(bot, msg).await?;
            }
            Command::PairDepeg => {
                self.handle_pair_depeg(bot, msg).await?;
            }
            Command::Alerts => {
                self.handle_list_alerts(bot, msg).await?;
            }
            Command::Delete => {
                self.handle_delete_alert(bot, msg).await?;
            }
            Command::Symbols => {
                self.handle_symbols(bot, msg).await?;
            }
            Command::Balance => {
                match handlers::handle_balance(bot, msg, self.exchange_manager.clone()).await {
                    Ok(_) => Ok(()),
                    Err(e) => Err(RequestError::Api(ApiError::Unknown(e.to_string()))),
                }?;
            }
            Command::Order(text) => {
                handlers::handle_order(bot, msg, text, self.exchange_manager.clone()).await?;
            }
            Command::Orders => {
                handlers::handle_orders(bot, msg, self.exchange_manager.clone()).await?;
            }
        }
        Ok(())
    }

    async fn handle_callback(&self, bot: Bot, query: CallbackQuery) -> ResponseResult<()> {
        if let Some(data) = query.data {
            if let Some(message) = query.message {
                match data.as_str() {
                    "create_price_alert" => {
                        let state = UserState::CreatingPriceAlert {
                            step: PriceAlertStep::SelectSymbol,
                            symbol: None,
                            target_price: None,
                            condition: None,
                        };
                        self.db.save_user_state(message.chat.id.0, &state)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))?;
                        self.handle_price_alert_step(&bot, message, &state).await?;
                    }
                    "create_depeg_alert" => {
                        let state = UserState::CreatingDepegAlert {
                            step: DepegAlertStep::SelectSymbol,
                            symbol: None,
                            target_price: None,
                            differential: None,
                            exchanges: None,
                        };
                        self.db.save_user_state(message.chat.id.0, &state)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))?;
                        self.handle_depeg_alert_step(&bot, message, &state).await?;
                    }
                    "create_pair_alert" => {
                        let state = UserState::CreatingPairAlert {
                            step: PairAlertStep::SelectToken1,
                            token1: None,
                            token2: None,
                            expected_ratio: None,
                            differential: None,
                        };
                        self.db.save_user_state(message.chat.id.0, &state)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))?;
                        self.handle_pair_depeg_step(&bot, message, &state).await?;
                    }
                    s if s.starts_with("symbol_") => {
                        let symbol = s.trim_start_matches("symbol_").to_string();
                        if let Some(state) = self.db.get_user_state(message.chat.id.0)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))? {
                            match state {
                                UserState::CreatingPriceAlert { step: PriceAlertStep::SelectSymbol, .. } => {
                                    let new_state = UserState::CreatingPriceAlert {
                                        step: PriceAlertStep::EnterPrice,
                                        symbol: Some(symbol),
                                        target_price: None,
                                        condition: None,
                                    };
                                    self.db.save_user_state(message.chat.id.0, &new_state)
                                        .await
                                        .map_err(|e| Self::db_error_to_request_error(e))?;
                                    self.handle_price_alert_step(&bot, message, &new_state).await?;
                                }
                                UserState::CreatingDepegAlert { step: DepegAlertStep::SelectSymbol, .. } => {
                                    let new_state = UserState::CreatingDepegAlert {
                                        step: DepegAlertStep::EnterDifferential,
                                        symbol: Some(symbol.clone()),
                                        target_price: Some(1.0), // Siempre $1 para stablecoins
                                        differential: None,
                                        exchanges: None,
                                    };
                                    
                                    self.db.save_user_state(message.chat.id.0, &new_state)
                                        .await
                                        .map_err(|e| Self::db_error_to_request_error(e))?;
                                    
                                    self.handle_depeg_alert_step(&bot, message, &new_state).await?;
                                }
                                // Manejar otros estados similares para depeg y pair alerts
                                _ => {
                                    info!("Estado no esperado para symbol callback");
                                }
                            }
                        }
                    }
                    s if s.starts_with("condition_") => {
                        let condition = match s.trim_start_matches("condition_") {
                            "above" => AlertCondition::Above,
                            "below" => AlertCondition::Below,
                            _ => {
                                info!("Condición no válida");
                                return Ok(());
                            }
                        };
                        
                        if let Some(state) = self.db.get_user_state(message.chat.id.0)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))? {
                            if let UserState::CreatingPriceAlert { symbol, target_price, .. } = state {
                                if let (Some(symbol), Some(price)) = (symbol, target_price) {
                                    // Obtener usuario
                                    let _user = self.get_user_by_chat_id(message.chat.id.0).await?;

                                    // Crear la alerta
                                    let alert = PriceAlert {
                                        id: None,
                                        user_id: _user.id,
                                        symbol: symbol.clone(),
                                        alert_type: AlertType::Price {
                                            target_price: price,
                                            condition: condition.clone(),
                                        },
                                        created_at: Some(chrono::Utc::now().timestamp()),
                                        triggered_at: None,
                                        is_active: true,
                                    };

                                    // Clonar los valores necesarios antes de mover alert
                                    let symbol = alert.symbol.clone();
                                    let target_price = if let AlertType::Price { target_price, .. } = alert.alert_type {
                                        target_price
                                    } else {
                                        0.0
                                    };
                                    let condition = if let AlertType::Price { condition, .. } = &alert.alert_type {
                                        condition.clone()
                                    } else {
                                        AlertCondition::Above
                                    };

                                    // Guardar la alerta
                                    match self.db.save_alert(alert).await {
                                        Ok(_) => {
                                            bot.send_message(
                                                message.chat.id,
                                                format!(
                                                    "✅ Alerta creada exitosamente!\n\n\
                                                     Símbolo: {}\n\
                                                     Precio objetivo: ${:.2}\n\
                                                     Condición: {:?}",
                                                    symbol, target_price, condition
                                                )
                                            ).await?;
                                            
                                            // Limpiar el estado del usuario
                                            self.db.clear_user_state(message.chat.id.0)
                                                .await
                                                .map_err(|e| Self::db_error_to_request_error(e))?;
                                        }
                                        Err(e) => {
                                            error!("Error al crear alerta: {}", e);
                                            bot.send_message(message.chat.id, "❌ Error al crear la alerta").await?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    s if s.starts_with("delete_") => {
                        let alert_id = s.trim_start_matches("delete_")
                            .parse::<i64>()
                            .map_err(|_| RequestError::Api(ApiError::Unknown("ID inválido".to_string())))?;

                        // Verificar que el usuario es dueño de la alerta
                        let _user = self.get_user_by_chat_id(message.chat.id.0).await?;

                        match self.db.delete_alert(alert_id).await {
                            Ok(_) => {
                                bot.send_message(
                                    message.chat.id,
                                    format!("✅ Alerta #{} eliminada exitosamente", alert_id)
                                ).await?;
                            }
                            Err(e) => {
                                error!("Error al eliminar alerta: {}", e);
                                bot.send_message(message.chat.id, "❌ Error al eliminar la alerta").await?;
                            }
                        }
                    }
                    s if s.starts_with("pair_") => {
                        let pair = s.trim_start_matches("pair_").replace("_", "/");
                        if let Some(state) = self.db.get_user_state(message.chat.id.0)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))? {
                            if let UserState::CreatingPairAlert { step: PairAlertStep::SelectToken1, .. } = state {
                                let (token1, token2) = pair.split_once('/')
                                    .ok_or_else(|| RequestError::Api(ApiError::Unknown("Par inválido".to_string())))?;

                                let new_state = UserState::CreatingPairAlert {
                                    step: PairAlertStep::EnterRatio,
                                    token1: Some(token1.to_string()),
                                    token2: Some(token2.to_string()),
                                    expected_ratio: None,
                                    differential: None,
                                };

                                self.db.save_user_state(message.chat.id.0, &new_state)
                                    .await
                                    .map_err(|e| Self::db_error_to_request_error(e))?;

                                bot.send_message(
                                    message.chat.id,
                                    format!("Has seleccionado el par {}/{}.\nPor favor, ingresa el ratio esperado (ejemplo: 1.0):", 
                                        token1, token2)
                                ).await?;
                            }
                        }
                    }
                    s if s.starts_with("pairdiff_") => {
                        let diff: f64 = s.trim_start_matches("pairdiff_")
                            .parse()
                            .map_err(|_| RequestError::Api(ApiError::Unknown("Diferencial inválido".to_string())))?;

                        if let Some(state) = self.db.get_user_state(message.chat.id.0)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))? {
                            if let UserState::CreatingPairAlert { token1, token2, expected_ratio, .. } = state {
                                if let (Some(token1), Some(token2), Some(ratio)) = (token1, token2, expected_ratio) {
                                    // Obtener usuario
                                    let _user = self.get_user_by_chat_id(message.chat.id.0).await?;

                                    // Crear la alerta
                                    let alert = PriceAlert {
                                        id: None,
                                        user_id: _user.id,
                                        symbol: format!("{}/{}", &token1, &token2),
                                        alert_type: AlertType::PairDepeg {
                                            token1: token1.clone(),
                                            token2: token2.clone(),
                                            expected_ratio: ratio,
                                            differential: diff,
                                        },
                                        created_at: Some(chrono::Utc::now().timestamp()),
                                        triggered_at: None,
                                        is_active: true,
                                    };

                                    // Guardar la alerta
                                    match self.db.save_alert(alert).await {
                                        Ok(_) => {
                                            bot.send_message(
                                                message.chat.id,
                                                format!(
                                                    "✅ Alerta de par creada exitosamente!\n\n\
                                                     Par: {}/{}\n\
                                                     Ratio esperado: {}\n\
                                                     Diferencial: {}%",
                                                    token1, token2, ratio, diff
                                                )
                                            ).await?;
                                            
                                            // Limpiar el estado del usuario
                                            self.db.clear_user_state(message.chat.id.0)
                                                .await
                                                .map_err(|e| Self::db_error_to_request_error(e))?;
                                        }
                                        Err(e) => {
                                            error!("Error al crear alerta: {}", e);
                                            bot.send_message(message.chat.id, "❌ Error al crear la alerta").await?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    s if s.starts_with("depegdiff_") => {
                        let diff: f64 = s.trim_start_matches("depegdiff_")
                            .parse()
                            .map_err(|_| RequestError::Api(ApiError::Unknown("Diferencial inválido".to_string())))?;

                        if let Some(state) = self.db.get_user_state(message.chat.id.0)
                            .await
                            .map_err(|e| Self::db_error_to_request_error(e))? {
                            if let UserState::CreatingDepegAlert { symbol, .. } = state {
                                if let Some(symbol) = symbol {
                                    let _user = self.get_user_by_chat_id(message.chat.id.0).await?;

                                    // Crear alerta de depeg
                                    let alert = PriceAlert {
                                        id: None,
                                        user_id: _user.id,
                                        symbol: symbol.clone(),
                                        alert_type: AlertType::Depeg {
                                            target_price: 1.0,  // Siempre $1 para stablecoins
                                            differential: diff,
                                            exchanges: vec!["binance".to_string(), "coinbase".to_string()]
                                        },
                                        created_at: Some(chrono::Utc::now().timestamp()),
                                        triggered_at: None,
                                        is_active: true,
                                    };

                                    match self.db.save_alert(alert).await {
                                        Ok(_) => {
                                            bot.send_message(
                                                message.chat.id,
                                                format!(
                                                    "✅ Alerta de depeg creada!\n\n\
                                                     Stablecoin: {}\n\
                                                     Se alertará si se desvía más de {}% de $1",
                                                    symbol, diff
                                                )
                                            ).await?;
                                            
                                            self.db.clear_user_state(message.chat.id.0)
                                                .await
                                                .map_err(|e| Self::db_error_to_request_error(e))?;
                                        }
                                        Err(e) => {
                                            error!("Error al crear alerta: {}", e);
                                            bot.send_message(message.chat.id, "❌ Error al crear la alerta").await?;
                                        }
                                    }
                                }
                            }
                        }
                    }
                    _ => {
                        info!("Callback no manejado: {}", data);
                    }
                }
                
                // Responder al callback query para quitar el estado de "loading"
                bot.answer_callback_query(query.id).await?;
            }
        } 
        Ok(())
    }

    async fn handle_register(&self, bot: Bot, msg: Message, username: String, password: String) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Intento de registro: username={}, chat_id={}", username, chat_id);
        
        let auth = Auth::new(self.db.as_ref());
        
        match auth.register_user(&username, &password).await {
            Ok(user) => {
                if let Err(e) = self.db.update_user_telegram_chat_id(user.id, chat_id).await {
                    let error_msg = format!("Error al actualizar chat_id: {}", e);
                    error!("{}", error_msg);
                    bot.send_message(msg.chat.id, "Error al vincular cuenta con Telegram").await?;
                    return Ok(());
                }

                match self.db.create_api_key(user.id).await {
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

    async fn handle_alert(&self, bot: Bot, msg: Message, symbol: String, price: f64, condition_str: String) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        
        // Obtener usuario por chat_id
        let user = self.get_user_by_chat_id(chat_id).await?;

        // Validar condición
        let condition = match condition_str.to_lowercase().as_str() {
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
            alert_type: AlertType::Price {
                target_price: price,
                condition,
            },
            created_at: Some(chrono::Utc::now().timestamp()),
            triggered_at: None,
            is_active: true,
        };

        // Clonar los valores necesarios antes de mover alert
        let symbol = alert.symbol.clone();
        let target_price = if let AlertType::Price { target_price, .. } = alert.alert_type {
            target_price
        } else {
            0.0
        };
        let condition = if let AlertType::Price { condition, .. } = &alert.alert_type {
            condition.clone()
        } else {
            AlertCondition::Above
        };

        match self.db.save_alert(alert).await {
            Ok(_) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "✅ Alerta creada exitosamente!\n\n\
                         Símbolo: {}\n\
                         Precio objetivo: ${:.2}\n\
                         Condición: {:?}",
                        symbol, target_price, condition
                    )
                ).await?;
            }
            Err(e) => {
                error!("Error al crear alerta: {}", e);
                bot.send_message(msg.chat.id, "❌ Error al crear la alerta").await?;
            }
        }

        Ok(())
    }

    async fn handle_depeg(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let state = UserState::CreatingDepegAlert {
            step: DepegAlertStep::SelectSymbol,
            symbol: None,
            target_price: None,
            differential: None,
            exchanges: None,
        };
        self.db.save_user_state(msg.chat.id.0, &state)
            .await
            .map_err(|e| Self::db_error_to_request_error(e))?;
        self.handle_depeg_alert_step(&bot, msg, &state).await?;
        Ok(())
    }

    async fn handle_pair_depeg(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let state = UserState::CreatingPairAlert {
            step: PairAlertStep::SelectToken1,
            token1: None,
            token2: None,
            expected_ratio: None,
            differential: None,
        };
        self.db.save_user_state(msg.chat.id.0, &state)
            .await
            .map_err(|e| Self::db_error_to_request_error(e))?;
        self.handle_pair_depeg_step(&bot, msg, &state).await?;
        Ok(())
    }

    async fn handle_pair_depeg_step(&self, bot: &Bot, msg: Message, state: &UserState) -> ResponseResult<()> {
        if let UserState::CreatingPairAlert { step, .. } = state {
            match step {
                PairAlertStep::SelectToken1 => {
                    // Mostrar pares predefinidos desde config
                    let pairs = [
                        ["BTC/WBTC", "BTC/renBTC", "BTC/sBTC"],
                        ["ETH/stETH", "ETH/rETH", "ETH/cbETH"],
                        ["SOL/mSOL", "ATOM/stATOM", "INJ/sINJ"],
                    ];
                    
                    let markup = InlineKeyboardMarkup::new(
                        pairs.iter().map(|row| {
                            row.iter().map(|&pair| {
                                InlineKeyboardButton::callback(pair, format!("pair_{}", pair.replace("/", "_")))
                            }).collect::<Vec<_>>()
                        })
                    );

                    bot.send_message(
                        msg.chat.id,
                        "Selecciona el par de tokens a monitorear:",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                PairAlertStep::EnterRatio => {
                    bot.send_message(
                        msg.chat.id,
                        "Por favor, ingresa el ratio esperado (ejemplo: 1.0):\n\n\
                         Nota: Un ratio de 2.0 significa que 1 token base = 2 tokens sintéticos",
                    ).await?;
                },
                PairAlertStep::EnterDifferential => {
                    let markup = InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("0.5%", "pairdiff_0.5"),
                        InlineKeyboardButton::callback("1%", "pairdiff_1"),
                        InlineKeyboardButton::callback("2%", "pairdiff_2"),
                        InlineKeyboardButton::callback("5%", "pairdiff_5"),
                    ]]);

                    bot.send_message(
                        msg.chat.id,
                        "Selecciona el porcentaje de desviación permitido:",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                _ => {}
            }
        }
        Ok(())
    }

    // Método auxiliar para obtener usuario por chat_id
    async fn get_user_by_chat_id(&self, chat_id: i64) -> ResponseResult<User> {
        match self.db.get_user_by_telegram_id(chat_id).await {
            Ok(Some(user)) => Ok(user),
            Ok(None) => Err(RequestError::Api(ApiError::Unknown("Usuario no encontrado".to_string()))),
            Err(e) => Err(RequestError::Api(ApiError::Unknown(e.to_string())))
        }
    }

    async fn handle_list_alerts(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        info!("Listando alertas para chat_id={}", chat_id);

        // Obtener usuario por chat_id
        let user = self.get_user_by_chat_id(chat_id).await?;

        // Obtener alertas del usuario
        match self.db.get_user_alerts(user.id).await {
            Ok(alerts) => {
                if alerts.is_empty() {
                    bot.send_message(msg.chat.id, "No tienes alertas configuradas").await?;
                    return Ok(());
                }

                let mut response = String::from("📊 Tus alertas:\n\n");
                for alert in alerts {
                    let status = if alert.is_active { "🟢 Activa" } else { "🔴 Disparada" };
                    
                    let alert_details = match &alert.alert_type {
                        AlertType::Price { target_price, condition } => {
                            format!(
                                "ID: {}\n\
                                 Tipo: Precio\n\
                                 Símbolo: {}\n\
                                 Precio objetivo: ${:.2}\n\
                                 Condición: {:?}\n\
                                 Estado: {}\n",
                                alert.id.unwrap_or(-1),
                                alert.symbol,
                                target_price,
                                condition,
                                status
                            )
                        },
                        AlertType::Depeg { target_price, differential, exchanges } => {
                            format!(
                                "ID: {}\n\
                                 Tipo: Depeg\n\
                                 Símbolo: {}\n\
                                 Precio objetivo: ${:.2}\n\
                                 Diferencial: {:.2}%\n\
                                 Exchanges: {}\n\
                                 Estado: {}\n",
                                alert.id.unwrap_or(-1),
                                alert.symbol,
                                target_price,
                                differential,
                                exchanges.join(", "),
                                status
                            )
                        },
                        AlertType::PairDepeg { token1, token2, expected_ratio, differential } => {
                            format!(
                                "ID: {}\n\
                                 Tipo: Depeg de Par\n\
                                 Par: {}/{}\n\
                                 Ratio esperado: {:.4}\n\
                                 Diferencial: {:.2}%\n\
                                 Estado: {}\n",
                                alert.id.unwrap_or(-1),
                                token1, token2,
                                expected_ratio,
                                differential,
                                status
                            )
                        }
                    };
                    
                    response.push_str(&alert_details);
                    response.push_str("\n");
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

    async fn handle_delete_alert(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let chat_id = msg.chat.id.0;
        
        // Obtener usuario
        let user = self.get_user_by_chat_id(chat_id).await?;

        // Obtener alertas activas
        match self.db.get_user_alerts(user.id).await {
            Ok(alerts) if !alerts.is_empty() => {
                let keyboard: Vec<Vec<InlineKeyboardButton>> = alerts.iter().map(|alert| {
                    let description = match &alert.alert_type {
                        AlertType::Price { target_price, condition } => {
                            format!("ID {}: {} ${} {:?}", alert.id.unwrap_or(-1), alert.symbol, target_price, condition)
                        },
                        AlertType::Depeg { target_price, differential, .. } => {
                            format!("ID {}: {} ${} (±{}%)", alert.id.unwrap_or(-1), alert.symbol, target_price, differential)
                        },
                        AlertType::PairDepeg { token1, token2, expected_ratio, differential } => {
                            format!("ID {}: {}/{} ratio {} (±{}%)", 
                                alert.id.unwrap_or(-1), token1, token2, expected_ratio, differential)
                        }
                    };
                    vec![InlineKeyboardButton::callback(description, format!("delete_{}", alert.id.unwrap_or(-1)))]
                }).collect();

                let markup = InlineKeyboardMarkup::new(keyboard);
                bot.send_message(msg.chat.id, "Selecciona la alerta que deseas eliminar:")
                    .reply_markup(markup)
                    .await?;
            },
            Ok(_) => {
                bot.send_message(msg.chat.id, "No tienes alertas activas.").await?;
            },
            Err(e) => {
                error!("Error al obtener alertas: {}", e);
                bot.send_message(msg.chat.id, "❌ Error al obtener las alertas").await?;
            }
        }
        Ok(())
    }

    async fn handle_symbols(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        info!("Mostrando símbolos soportados");
        
        let symbols: Vec<String> = CONFIG.cryptocurrencies.iter()
            .map(|symbol| symbol.clone())
            .collect();

        let response = format!(
            "🪙 Símbolos soportados:\n\n{}\n\n\
             Uso: /alert <symbol> <price> <above|below>\n\
             Ejemplo: /alert BTC 42000 above",
            symbols.join("\n")
        );

        bot.send_message(msg.chat.id, response).await?;
        Ok(())
    }

    async fn handle_alert_creation(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        info!("Iniciando creación de alerta");
        
        let markup = InlineKeyboardMarkup::new([[
            InlineKeyboardButton::callback("💰 Precio", "create_price_alert"),
            InlineKeyboardButton::callback("🎯 Depeg", "create_depeg_alert"),
            InlineKeyboardButton::callback("⚖️ Par de Tokens", "create_pair_alert"),
        ]]);

        bot.send_message(
            msg.chat.id,
            "¿Qué tipo de alerta quieres crear?",
        )
        .reply_markup(markup)
        .await?;

        info!("Mensaje de selección de alerta enviado");
        Ok(())
    }

    async fn handle_price_alert_step(&self, bot: &Bot, msg: Message, state: &UserState) -> ResponseResult<()> {
        if let UserState::CreatingPriceAlert { step, .. } = state {
            match step {
                PriceAlertStep::SelectSymbol => {
                    let supported_symbols = CONFIG.get_supported_symbols();
                    let symbols: Vec<Vec<String>> = supported_symbols
                        .chunks(3)
                        .map(|chunk| chunk.to_vec())
                        .collect();
                    
                    let markup = InlineKeyboardMarkup::new(
                        symbols.iter().map(|row| {
                            row.iter().map(|symbol| {
                                InlineKeyboardButton::callback(symbol, format!("symbol_{}", symbol))
                            }).collect::<Vec<_>>()
                        })
                    );

                    bot.send_message(
                        msg.chat.id,
                        "Selecciona la criptomoneda que quieres monitorear:",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                PriceAlertStep::EnterPrice => {
                    bot.send_message(
                        msg.chat.id,
                        "Ingresa el precio objetivo (ejemplo: 45000.50):",
                    ).await?;
                },
                PriceAlertStep::SelectCondition => {
                    let markup = InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("⬆️ Por encima", "condition_above"),
                        InlineKeyboardButton::callback("⬇️ Por debajo", "condition_below"),
                    ]]);

                    bot.send_message(
                        msg.chat.id,
                        "¿Cuándo quieres recibir la alerta?",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                PriceAlertStep::Confirm => {
                    // Mostrar resumen y botones de confirmar/cancelar
                }
            }
        }
        Ok(())
    }

    async fn handle_depeg_alert_step(&self, bot: &Bot, msg: Message, state: &UserState) -> ResponseResult<()> {
        if let UserState::CreatingDepegAlert { step, .. } = state {
            match step {
                DepegAlertStep::SelectSymbol => {
                    // Solo mostrar stablecoins
                    let stablecoins = [
                        ["USDT", "USDC", "BUSD"],
                        ["DAI", "FRAX", "LUSD"],
                        ["MAI", "USDD", "sUSD"],
                    ];
                    
                    let markup = InlineKeyboardMarkup::new(
                        stablecoins.iter().map(|row| {
                            row.iter().map(|&coin| {
                                InlineKeyboardButton::callback(coin, format!("symbol_{}", coin))
                            }).collect::<Vec<_>>()
                        })
                    );

                    bot.send_message(
                        msg.chat.id,
                        "Selecciona la stablecoin a monitorear:\n\
                         (Se alertará cuando se desvíe de $1)",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                DepegAlertStep::EnterDifferential => {
                    let markup = InlineKeyboardMarkup::new([[
                        InlineKeyboardButton::callback("0.5%", "depegdiff_0.5"),
                        InlineKeyboardButton::callback("1%", "depegdiff_1"),
                        InlineKeyboardButton::callback("2%", "depegdiff_2"),
                        InlineKeyboardButton::callback("5%", "depegdiff_5"),
                    ]]);

                    bot.send_message(
                        msg.chat.id,
                        "¿Cuánta desviación del peg ($1) quieres permitir antes de recibir una alerta?",
                    )
                    .reply_markup(markup)
                    .await?;
                },
                _ => {}
            }
        }
        Ok(())
    }

    async fn send_help_message(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        let help_text = r#"🤖 *Bot de Alertas Crypto*

*Tipos de Alertas:*
1️⃣ *Alerta de Precio*
   Notifica cuando un activo alcanza cierto precio
   Comando: /alert o usar menú interactivo

2️⃣ *Alerta de Depeg*
   Monitorea desviaciones de stablecoins
   Comando: /depeg o usar menú interactivo

3️⃣ *Alerta de Par*
   Vigila la relación entre dos tokens
   Comando: /pairdepeg o usar menú interactivo

*Comandos Principales:*
• /start \- Inicia el bot
• /help \- Muestra este mensaje
• /alerts \- Lista tus alertas activas
• /delete \- Elimina una alerta

*Consejos:*
• Usa los botones interactivos para crear alertas
• Los precios se manejan en USD
• Los diferenciales son porcentajes \(1 = 1%\)
• Puedes seleccionar múltiples exchanges

Para crear una alerta, usa /alert y sigue las instrucciones\."#;

        bot.send_message(msg.chat.id, help_text)
            .parse_mode(ParseMode::MarkdownV2)
            .await?;

        Ok(())
    }

    async fn handle_message(&self, bot: Bot, msg: Message) -> ResponseResult<()> {
        if let Some(text) = msg.text() {
            if let Some(state) = self.db.get_user_state(msg.chat.id.0)
                .await
                .map_err(|e| Self::db_error_to_request_error(e))? {
                match state {
                    UserState::CreatingPriceAlert { step: PriceAlertStep::EnterPrice, symbol, .. } => {
                        match text.parse::<f64>() {
                            Ok(price) => {
                                let new_state = UserState::CreatingPriceAlert {
                                    step: PriceAlertStep::SelectCondition,
                                    symbol,
                                    target_price: Some(price),
                                    condition: None,
                                };
                                self.db.save_user_state(msg.chat.id.0, &new_state)
                                    .await
                                    .map_err(|e| Self::db_error_to_request_error(e))?;
                                self.handle_price_alert_step(&bot, msg, &new_state).await?;
                            }
                            Err(_) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "❌ Precio inválido. Por favor, ingresa un número válido (ejemplo: 45000.50):"
                                ).await?;
                            }
                        }
                    }
                    UserState::CreatingPairAlert { step: PairAlertStep::EnterRatio, token1, token2, .. } => {
                        match text.parse::<f64>() {
                            Ok(ratio) => {
                                let new_state = UserState::CreatingPairAlert {
                                    step: PairAlertStep::EnterDifferential,
                                    token1,
                                    token2,
                                    expected_ratio: Some(ratio),
                                    differential: None,
                                };
                                self.db.save_user_state(msg.chat.id.0, &new_state)
                                    .await
                                    .map_err(|e| Self::db_error_to_request_error(e))?;

                                let markup = InlineKeyboardMarkup::new([[
                                    InlineKeyboardButton::callback("0.5%", "pairdiff_0.5"),
                                    InlineKeyboardButton::callback("1%", "pairdiff_1"),
                                    InlineKeyboardButton::callback("2%", "pairdiff_2"),
                                    InlineKeyboardButton::callback("5%", "pairdiff_5"),
                                ]]);

                                bot.send_message(
                                    msg.chat.id,
                                    "Selecciona el porcentaje de desviación permitido:"
                                )
                                .reply_markup(markup)
                                .await?;
                            }
                            Err(_) => {
                                bot.send_message(
                                    msg.chat.id,
                                    "❌ Ratio inválido. Por favor, ingresa un número válido (ejemplo: 1.0):"
                                ).await?;
                            }
                        }
                    }
                    // Manejar otros estados que requieren entrada de texto
                    _ => {}
                }
            }
        }
        Ok(())
    }

    pub async fn run(&self) {
        let handler = dptree::entry()
            .branch(
                Update::filter_message()
                    .filter_command::<Command>()
                    .endpoint({
                        let bot_instance = self.clone();
                        move |bot: Bot, msg: Message, cmd: Command| {
                            let handler = bot_instance.clone();
                            async move {
                                if let Err(e) = handler.handle_command(bot, msg, cmd).await {
                                    error!("Error manejando comando: {}", e);
                                    return Err(e);
                                }
                                Ok(())
                            }
                        }
                    })
            )
            .branch(
                Update::filter_message()
                    .endpoint({
                        let bot_instance = self.clone();
                        move |bot: Bot, msg: Message| {
                            let handler = bot_instance.clone();
                            async move {
                                if let Err(e) = handler.handle_message(bot, msg).await {
                                    error!("Error manejando mensaje: {}", e);
                                    return Err(e);
                                }
                                Ok(())
                            }
                        }
                    })
            )
            .branch(
                Update::filter_callback_query()
                    .endpoint({
                        let bot_instance = self.clone();
                        move |bot: Bot, q: CallbackQuery| {
                            let handler = bot_instance.clone();
                            async move {
                                if let Err(e) = handler.handle_callback(bot, q).await {
                                    error!("Error manejando callback: {}", e);
                                    return Err(e);
                                }
                                Ok(())
                            }
                        }
                    })
            );

        Dispatcher::builder(self.bot.clone(), handler)
            .dependencies(dptree::deps![Arc::clone(&self.db), Arc::clone(&self.exchange_manager)])
            .enable_ctrlc_handler()
            .build()
            .dispatch()
            .await;
    }

    pub async fn verify_bot_token() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let token = std::env::var("TELEGRAM_BOT_TOKEN")?;
        let bot = Bot::new(token);
        
        match bot.get_me().await {
            Ok(me) => {
                info!("Bot verificado: @{}", me.username());
                Ok(())
            }
            Err(e) => {
                error!("Error verificando bot: {}", e);
                Err(Box::new(e))
            }
        }
    }

    pub async fn handle_price_alert(&self, msg: Message, args: Vec<String>) -> Result<(), BotError> {
        match self.db.get_user_by_telegram_id(msg.chat.id.0).await {
            Ok(Some(user)) => {
                let alert = PriceAlert {
                    id: None,
                    user_id: user.id,
                    symbol: args[0].clone(),
                    alert_type: AlertType::Price {
                        target_price: args[1].parse()?,
                        condition: AlertCondition::Above,
                    },
                    created_at: Some(chrono::Utc::now().timestamp()),
                    triggered_at: None,
                    is_active: true,
                };

                // Clonar los valores necesarios antes de mover alert
                let symbol = alert.symbol.clone();
                let target_price = if let AlertType::Price { target_price, .. } = alert.alert_type {
                    target_price
                } else {
                    0.0
                };
                let condition = if let AlertType::Price { condition, .. } = &alert.alert_type {
                    condition.clone()
                } else {
                    AlertCondition::Above
                };

                match self.db.save_alert(alert).await {
                    Ok(_) => {
                        self.bot.send_message(
                            msg.chat.id,
                            format!(
                                "✅ Alerta de precio creada exitosamente\n\n\
                                 Símbolo: {}\n\
                                 Precio objetivo: ${:.2}\n\
                                 Condición: {:?}",
                                symbol, target_price, condition
                            )
                        ).await?;
                    }
                    Err(e) => {
                        error!("Error al guardar alerta: {}", e);
                        self.bot.send_message(
                            msg.chat.id,
                            "❌ Error al crear alerta"
                        ).await?;
                    }
                }
            }
            Ok(None) => {
                self.bot.send_message(
                    msg.chat.id,
                    "❌ Usuario no encontrado"
                ).await?;
            }
            Err(e) => {
                error!("Error al buscar usuario: {}", e);
                self.bot.send_message(
                    msg.chat.id,
                    "❌ Error interno del servidor"
                ).await?;
            }
        }
        Ok(())
    }
} 