use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{Message, ParseMode, InlineKeyboardButton, InlineKeyboardMarkup},
    RequestError,
    ApiError,
};
use crate::{
    exchanges::{
        types::{OrderSide, OrderType, ExchangeCredentials},
        ExchangeType,
        ExchangeManager,
        OrderRequest,
    },
    db::Database,
    bot::TelegramBot,
    config::crypto_config::{
        CRYPTO_CONFIG,
        get_supported_stablecoins,
        get_supported_pairs
    },
    models::{
        AlertCondition,
        PriceAlert,
    },
    exchanges::types::Exchange,
};
use rust_decimal::Decimal;
use std::{sync::Arc, str::FromStr};
use tracing::{error, info};

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;
pub type ResponseResult<T> = Result<T, Box<dyn std::error::Error + Send + Sync>>;

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

pub async fn handle_start(bot: Bot, msg: Message, _db: Arc<Database>) -> ResponseResult<()> {
    let text = "\
🎉 *¡BIENVENIDO AL BOT DE TRADING Y ALERTAS\\!*\n\
\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
💹 *TRADING EN BINANCE*\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
• Operar en múltiples pares\n\
• Ver balances y órdenes\n\
• Trading spot\n\
\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
⚠️ *SISTEMA DE ALERTAS*\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
• Alertas de precio\n\
• Monitoreo de stablecoins\n\
• Alertas de pares\n\
\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
*PRIMEROS PASOS:*\n\
▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
1\\. Usa `/register` para crear una cuenta\n\
2\\. Configura tus credenciales de Binance\n\
3\\. ¡Empieza a operar\\!\n\
\n\
Usa `/help` para ver todos los comandos\\.";
    
    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
}

pub async fn handle_help(bot: Bot, msg: Message) -> ResponseResult<()> {
    let formatted_text = format!(
        "🤖 *BOT DE TRADING Y ALERTAS*\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         📋 *COMANDOS BÁSICOS*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         [/help] \\- Ver este menú\n\
         [/start] \\- Iniciar el bot\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         💹 *TRADING EN BINANCE*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         [/balance] \\- Ver balance de la cuenta\n\
         [/order] \\- Crear orden de trading\n\
         [/orders] \\- Ver órdenes abiertas\n\
         [/cancel] \\- Cancelar una orden\n\
         [/symbols] \\- Ver pares disponibles\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         ⚠️ *SISTEMA DE ALERTAS*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         [/alert] \\- Crear alerta de precio\n\
         [/depeg] \\- Alerta de depeg stablecoin\n\
         [/pairdepeg] \\- Alerta de par\n\
         [/alerts] \\- Ver alertas activas\n\
         [/delete] \\- Eliminar una alerta\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         🔐 *CONFIGURACIÓN*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         [/register] \\- Registrar usuario\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         ℹ️ *INFORMACIÓN*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         • Para más detalles sobre un comando,\n
           usa: `<comando> help`\n\
           Ejemplo: [/order help]\n\
         \n\
         • Pares soportados: [/symbols]\n\
         • Estado: Monitoreando {} pares\n\
         • Intervalo: {} segundos",
        crate::exchanges::get_all_pairs().len(),
        50 // valor por defecto
    );

    bot.send_message(msg.chat.id, formatted_text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

pub async fn handle_balance(bot: Bot, msg: Message, exchange_manager: Arc<ExchangeManager>) -> ResponseResult<()> {
    let response = match exchange_manager.get_balances().await {
        Ok(balances) => {
            let mut message = String::from("*Balances*\n\n");
            for balance in balances {
                message.push_str(&format!(
                    "*{}*\n\
                     Free: `{:.8}`\n\
                     Locked: `{:.8}`\n\
                     Total: `{:.8}`\n\
                     \\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\n",
                    escape_markdown(&balance.asset),
                    balance.free,
                    balance.locked,
                    balance.free + balance.locked
                ));
            }
            message
        },
        Err(e) => format!("❌ Error al obtener balances: {}", escape_markdown(&e.to_string())),
    };

    bot.send_message(msg.chat.id, response)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

pub async fn handle_order(bot: Bot, msg: Message, text: String, exchange_manager: Arc<ExchangeManager>) -> ResponseResult<()> {
    let args: Vec<&str> = text.trim().split_whitespace().collect();
    
    // Mostrar ayuda si se solicita
    if args.is_empty() || args[0] == "help" {
        return handle_order_help(bot, msg).await;
    }

    info!("Argumentos recibidos: {:?}", args);
    
    if args.len() < 4 {
        bot.send_message(
            msg.chat.id,
            "Uso: /order <symbol> <side> <type> <quantity> [price]\n\
             Ejemplo market: /order RUNEUSDT buy market 0.001\n\
             Ejemplo limit: /order RUNEUSDT sell limit 0.001 4.05"
        ).await?;
        return Ok(());
    }

    let symbol = args[0].to_uppercase();
    info!("Symbol: {}", symbol);
    
    // Validar el par de trading
    if !crate::exchanges::is_valid_pair(&symbol) {
        let similar_pairs = crate::exchanges::get_similar_pairs(&symbol);
        let mut message = format!("❌ Par inválido: {}\n\nPares disponibles similares:\n", symbol);
        
        if similar_pairs.is_empty() {
            message.push_str("\nUsa /symbols para ver todos los pares disponibles");
        } else {
            for pair in similar_pairs {
                message.push_str(&format!("- {}\n", pair));
            }
        }
        
        bot.send_message(msg.chat.id, message).await?;
        return Ok(());
    }

    let side = match args[1].to_lowercase().as_str() {
        "buy" => OrderSide::Buy,
        "sell" => OrderSide::Sell,
        _ => {
            bot.send_message(msg.chat.id, "Side inválido. Use 'buy' o 'sell'").await?;
            return Ok(());
        }
    };

    let order_type = match args[2].to_lowercase().as_str() {
        "market" => OrderType::Market,
        "limit" => OrderType::Limit,
        _ => {
            bot.send_message(msg.chat.id, "Tipo inválido. Use 'market' o 'limit'").await?;
            return Ok(());
        }
    };

    let quantity = match Decimal::from_str(args[3]) {
        Ok(q) => q,
        Err(_) => {
            bot.send_message(msg.chat.id, "Cantidad inválida").await?;
            return Ok(());
        }
    };

    let price = if order_type == OrderType::Limit {
        if args.len() < 5 {
            bot.send_message(msg.chat.id, "Precio requerido para órdenes limit").await?;
            return Ok(());
        }
        match Decimal::from_str(args[4]) {
            Ok(p) => Some(p),
            Err(_) => {
                bot.send_message(msg.chat.id, "Precio inválido").await?;
                return Ok(());
            }
        }
    } else {
        None
    };

    info!("Creando orden: symbol={}, side={:?}, type={:?}, quantity={}, price={:?}", 
          symbol, side, order_type, quantity, price);

    let order_request = OrderRequest {
        symbol,
        side,
        order_type,
        quantity,
        price,
    };

    match exchange_manager.as_ref().place_order(
        &order_request.symbol,
        order_request.side,
        order_request.order_type,
        order_request.quantity,
        order_request.price,
    ).await {
        Ok(order) => {
            let msg_text = format!(
                "✅ Orden creada:\n\
                 ID: `{}`\n\
                 Symbol: `{}`\n\
                 Side: `{:?}`\n\
                 Type: `{:?}`\n\
                 Quantity: `{}`\n\
                 Price: `{}`\n\
                 Status: `{:?}`",
                order.id, order.symbol, order.side, order.order_type,
                order.quantity, order.price.unwrap_or_default(), order.status
            );
            bot.send_message(msg.chat.id, msg_text)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        Err(e) => {
            error!("Error al crear orden: {}", e);
            bot.send_message(
                msg.chat.id,
                format!("❌ Error al crear orden: {}", e)
            ).await?;
        }
    }

    Ok(())
}

pub async fn handle_orders(bot: Bot, msg: Message, exchange_manager: Arc<ExchangeManager>) -> ResponseResult<()> {
    match exchange_manager.get_open_orders(ExchangeType::Binance).await {
        Ok(orders) => {
            if orders.is_empty() {
                bot.send_message(msg.chat.id, "No hay órdenes abiertas")
                    .parse_mode(ParseMode::MarkdownV2)
                    .await?;
                return Ok(());
            }

            let mut message = String::from("*Órdenes Abiertas:*\n\n");
            for order in orders {
                message.push_str(&format!(
                    "*{}* \\- {}\n\
                     ID: `{}`\n\
                     Precio: `{}`\n\
                     Cantidad: `{:.8}`\n\
                     Estado: `{:?}`\n\
                     [Cancelar](/cancel {})\n\
                     \\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\\-\n",
                    escape_markdown(&order.symbol),
                    if order.side == OrderSide::Buy { "Compra" } else { "Venta" },
                    order.id,
                    order.price.map_or("Mercado".to_string(), |p| format!("{:.8}", p)),
                    order.quantity,
                    order.status,
                    order.id
                ));
            }

            bot.send_message(msg.chat.id, message)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("❌ Error al obtener órdenes: {}", escape_markdown(&e.to_string()))
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        }
    }

    Ok(())
}

pub async fn handle_cancel(
    bot: Bot, 
    msg: Message, 
    text: String,
    exchange_manager: Arc<ExchangeManager>
) -> ResponseResult<()> {
    // Si se proporciona un ID, cancelar esa orden específica
    if !text.is_empty() {
        return cancel_specific_order(bot, msg, text, exchange_manager).await;
    }

    // Obtener órdenes activas
    match exchange_manager.get_open_orders(ExchangeType::Binance).await {
        Ok(orders) => {
            if orders.is_empty() {
                bot.send_message(msg.chat.id, "No hay órdenes activas para cancelar.")
                    .await?;
                return Ok(());
            }

            // Crear teclado inline con las órdenes
            let keyboard: Vec<Vec<InlineKeyboardButton>> = orders
                .iter()
                .map(|order| {
                    vec![InlineKeyboardButton::callback(
                        format!(
                            "{} {} {} @ {} {}",
                            if order.side == OrderSide::Buy { "🟢" } else { "🔴" },
                            order.symbol,
                            order.quantity,
                            order.price.unwrap_or_default(),
                            order.order_type.to_string()
                        ),
                        format!("cancel_order:{}:{}", order.symbol, order.id)  // Incluir el símbolo
                    )]
                })
                .collect();

            let markup = InlineKeyboardMarkup::new(keyboard);

            bot.send_message(
                msg.chat.id,
                "Selecciona la orden que deseas cancelar:"
            )
            .reply_markup(markup)
            .await?;
        }
        Err(e) => {
            tracing::error!("Error al obtener órdenes: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error al obtener las órdenes activas. Por favor intente nuevamente."
            ).await?;
        }
    }

    Ok(())
}

// Handler para el callback de los botones
pub async fn handle_callback_query(
    bot: Bot,
    q: CallbackQuery,
    exchange_manager: Arc<ExchangeManager>
) -> ResponseResult<()> {
    if let Some(data) = q.data {
        if let Some(message) = q.message {
            if data.starts_with("cancel_order:") {
                let data_without_prefix = data.replace("cancel_order:", "");
                let parts: Vec<&str> = data_without_prefix.split(':').collect();
                if parts.len() != 2 {
                    tracing::error!("Formato inválido de callback data: {}", data);
                    return Ok(());
                }

                let symbol = parts[0];
                let order_id = parts[1];
                
                match exchange_manager.cancel_order(ExchangeType::Binance, symbol, order_id).await {
                    Ok(_) => {
                        // Actualizar el mensaje original
                        bot.edit_message_text(
                            message.chat.id,
                            message.id,
                            format!("✅ Orden {} cancelada exitosamente", order_id)
                        ).await?;

                        // Responder al callback
                        bot.answer_callback_query(q.id).await?;
                    }
                    Err(e) => {
                        tracing::error!("Error al cancelar orden {}: {}", order_id, e);
                        bot.answer_callback_query(q.id)
                            .text("❌ Error al cancelar la orden")
                            .show_alert(true)
                            .await?;
                    }
                }
            }
        }
    }

    Ok(())
}

async fn cancel_specific_order(
    bot: Bot,
    msg: Message,
    text: String,
    exchange_manager: Arc<ExchangeManager>
) -> ResponseResult<()> {
    let args: Vec<&str> = text.split_whitespace().collect();
    if args.len() != 2 {
        bot.send_message(
            msg.chat.id,
            "❌ Formato incorrecto. Uso: `/cancel <symbol> <order_id>`"
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    let symbol = args[0].to_uppercase();
    let order_id = args[1];

    match exchange_manager.cancel_order(ExchangeType::Binance, &symbol, order_id).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!("✅ Orden {} cancelada exitosamente", order_id)
            ).await?;
        }
        Err(e) => {
            tracing::error!("Error al cancelar orden {}: {}", order_id, e);
            bot.send_message(
                msg.chat.id,
                format!("❌ Error al cancelar la orden: {}", e)
            ).await?;
        }
    }
    Ok(())
}

// Función auxiliar para manejar la conexión de exchange
pub async fn handle_connect(
    bot: Bot,
    msg: Message,
    db: Arc<Database>,
    args: Vec<String>,
) -> ResponseResult<()> {
    match db.get_user_by_telegram_id(msg.chat.id.0).await {
        Ok(Some(user)) => {
            let credentials = ExchangeCredentials {
                api_key: args[1].clone(),
                api_secret: args[2].clone(),
            };
            
            match db.save_exchange_credentials(user.id, "binance", &credentials).await {
                Ok(_) => {
                    bot.send_message(
                        msg.chat.id,
                        "✅ Credenciales guardadas exitosamente"
                    ).await?;
                }
                Err(e) => {
                    error!("Error al guardar credenciales: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "❌ Error al guardar credenciales"
                    ).await?;
                }
            }
        }
        Ok(None) => {
            bot.send_message(
                msg.chat.id,
                "❌ Usuario no encontrado. Por favor registrate primero con /start"
            ).await?;
        }
        Err(e) => {
            error!("Error al buscar usuario: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error interno del servidor"
            ).await?;
        }
    }
    Ok(())
}

// Función auxiliar para manejar órdenes de compra
pub async fn handle_buy(
    bot: Bot,
    msg: Message,
    db: Arc<Database>,
    exchange_manager: Arc<ExchangeManager>,
    text: String
) -> ResponseResult<()> {
    let quantity = text.parse::<Decimal>()
        .map_err(|_| Box::new(ApiError::Unknown("Cantidad inválida".into())) as Box<dyn std::error::Error + Send + Sync>)?;

    let order_request = OrderRequest {
        symbol: "USDT".to_string(),
        side: OrderSide::Buy,
        order_type: OrderType::Market,
        quantity,
        price: None,
    };

    match db.get_user_by_telegram_id(msg.chat.id.0).await {
        Ok(Some(_user)) => {
            match exchange_manager.as_ref().place_order(
                &order_request.symbol,
                order_request.side,
                order_request.order_type,
                order_request.quantity,
                order_request.price,
            ).await {
                Ok(order) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "✅ Orden creada exitosamente!\n\
                             ID: {}\n\
                             Símbolo: {}\n\
                             Tipo: {}\n\
                             Cantidad: {}\n\
                             Precio: {}",
                            order.id,
                            order.symbol,
                            format!("{:?}", order.order_type),
                            order.quantity,
                            order.price.map_or("Mercado".into(), |p| p.to_string())
                        )
                    ).await?;
                }
                Err(e) => {
                    error!("Error al crear orden: {}", e);
                    bot.send_message(msg.chat.id, "❌ Error al crear orden").await?;
                }
            }
        }
        _ => {
            bot.send_message(msg.chat.id, "No estás autorizado. Usa /register primero").await?;
        }
    }

    Ok(())
}

pub async fn handle_symbols(bot: Bot, msg: Message) -> ResponseResult<()> {
    let pairs = crate::exchanges::get_all_pairs();
    let mut message = String::from("*Pares de Trading Disponibles*\\n\\n");
    
    for pair in pairs {
        message.push_str(&format!("\\- `{}`\\n", pair));
    }
    
    bot.send_message(msg.chat.id, message)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        
    Ok(())
}

// Y agregamos ayuda específica para el comando order
pub async fn handle_order_help(bot: Bot, msg: Message) -> ResponseResult<()> {
    let text = "*Comando /order \\- Crear órdenes de trading*\n\n\
                ━━━━━ SINTAXIS ━━━━━\n\
                `/order <symbol> <side> <type> <quantity> [price]`\n\n\
                ━━━━━ EJEMPLOS ━━━━━\n\
                Orden market: `/order RUNEUSDT buy market 40.60`\n\
                Orden limit: `/order RUNEUSDT sell limit 40.60 4.05`\n\n\
                ━━━━ PARÁMETROS ━━━━\n\
                `symbol` \\- Par de trading \\(ej: RUNEUSDT\\)\n\
                `side` \\- buy o sell\n\
                `type` \\- market o limit\n\
                `quantity` \\- Cantidad a operar\n\
                `price` \\- Precio \\(solo para órdenes limit\\)\n\n\
                Usa /symbols para ver los pares disponibles";

    bot.send_message(msg.chat.id, text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    Ok(())
}

pub async fn handle_sell(
    bot: Bot,
    msg: Message,
    db: Arc<Database>,
    exchange_manager: Arc<ExchangeManager>,
    args: String
) -> ResponseResult<()> {
    let parts: Vec<&str> = args.split_whitespace().collect();
    if parts.len() < 2 {
        bot.send_message(
            msg.chat.id,
            "❌ Uso: /sell <symbol> <quantity> [price]"
        ).await?;
        return Ok(());
    }

    let symbol = parts[0].to_uppercase();
    let quantity = match parts[1].parse::<Decimal>() {
        Ok(q) => q,
        Err(_) => {
            bot.send_message(msg.chat.id, "❌ Cantidad inválida").await?;
            return Ok(());
        }
    };

    // Obtener el precio actual del mercado
    let current_price = match exchange_manager.get_price(&symbol).await {
        Ok(price) => price,
        Err(e) => {
            error!("Error al obtener precio: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error al obtener el precio actual"
            ).await?;
            return Ok(());
        }
    };

    // Calcular el valor total de la orden
    let total_value = quantity * current_price;
    
    // Validar el valor mínimo (10 USDT para Binance)
    if total_value < Decimal::from(10) {
        bot.send_message(
            msg.chat.id,
            format!(
                "❌ El valor total de la orden ({} USDT) es menor que el mínimo permitido (10 USDT)",
                total_value
            )
        ).await?;
        return Ok(());
    }

    let price = parts.get(2).and_then(|p| p.parse::<Decimal>().ok());

    let order_request = OrderRequest {
        symbol,
        side: OrderSide::Sell,
        order_type: if price.is_some() { OrderType::Limit } else { OrderType::Market },
        quantity,
        price,
    };

    if let Ok(_user) = db.get_user_by_telegram_id(msg.chat.id.0).await {
        match exchange_manager.as_ref().place_order(
            &order_request.symbol,
            order_request.side,
            order_request.order_type,
            order_request.quantity,
            order_request.price,
        ).await {
            Ok(order) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "✅ Orden de venta creada exitosamente!\n\
                         ID: {}\n\
                         Símbolo: {}\n\
                         Tipo: {}\n\
                         Cantidad: {}\n\
                         Precio: {}\n\
                         Valor Total: {} USDT",
                        order.id,
                        order.symbol,
                        format!("{:?}", order.order_type),
                        order.quantity,
                        order.price.map_or("Mercado".into(), |p| p.to_string()),
                        total_value
                    )
                ).await?;
            }
            Err(e) => {
                error!("Error al crear orden: {}", e);
                bot.send_message(
                    msg.chat.id,
                    format!("❌ Error al crear orden: {}", e)
                ).await?;
            }
        }
    }

    Ok(())
}

impl TelegramBot {
    async fn handle_connect(&self, bot: Bot, msg: Message, args: String) -> ResponseResult<()> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        
        match parts.get(0).map(|s| s.to_lowercase()) {
            Some(exchange) if exchange == "binance" => {
                if parts.len() != 3 {
                    bot.send_message(
                        msg.chat.id,
                        "❌ Uso para Binance: /connect binance <api_key> <api_secret>"
                    ).await?;
                    return Ok(());
                }

                let api_key = parts[1];
                let api_secret = parts[2];

                let credentials = ExchangeCredentials {
                    api_key: api_key.to_string(),
                    api_secret: api_secret.to_string(),
                };

                // Guardar credenciales
                if let Ok(user) = self.get_user_by_chat_id(msg.chat.id.0).await {
                    match self.db.save_exchange_credentials(user.id, "binance", &credentials).await {
                        Ok(_) => {
                            bot.send_message(
                                msg.chat.id,
                                "✅ Binance conectado exitosamente!"
                            ).await?;
                        }
                        Err(e) => {
                            error!("Error al guardar credenciales: {}", e);
                            bot.send_message(msg.chat.id, "❌ Error al conectar Binance").await?;
                        }
                    }
                }
            }
            _ => {
                bot.send_message(
                    msg.chat.id,
                    "❌ Exchange no soportado. Usa 'binance'"
                ).await?;
            }
        }

        Ok(())
    }

    async fn handle_buy(&self, bot: Bot, msg: Message, args: String) -> ResponseResult<()> {
        let parts: Vec<&str> = args.split_whitespace().collect();
        if parts.len() < 3 {
            bot.send_message(
                msg.chat.id,
                "❌ Uso: /buy <symbol> <quantity> [price]"
            ).await?;
            return Ok(());
        }

        let symbol = parts[0].to_uppercase();
        let quantity = parts[1].parse::<Decimal>()
            .map_err(|_| RequestError::Api(ApiError::Unknown("Cantidad inválida".into())))?;
        let price = parts.get(2).and_then(|p| p.parse::<Decimal>().ok());

        let order_request = OrderRequest {
            symbol,
            side: OrderSide::Buy,
            order_type: if price.is_some() { OrderType::Limit } else { OrderType::Market },
            quantity,
            price,
        };

        if let Ok(_user) = self.get_user_by_chat_id(msg.chat.id.0).await {
            match self.exchange_manager.as_ref().place_order(
                &order_request.symbol,
                order_request.side,
                order_request.order_type,
                order_request.quantity,
                order_request.price,
            ).await {
                Ok(order) => {
                    bot.send_message(
                        msg.chat.id,
                        format!(
                            "✅ Orden creada exitosamente!\n\
                             ID: {}\n\
                             Símbolo: {}\n\
                             Tipo: {}\n\
                             Cantidad: {}\n\
                             Precio: {}",
                            order.id,
                            order.symbol,
                            format!("{:?}", order.order_type),
                            order.quantity,
                            order.price.map_or("Mercado".into(), |p| p.to_string())
                        )
                    ).await?;
                }
                Err(e) => {
                    error!("Error al crear orden: {}", e);
                    bot.send_message(msg.chat.id, "❌ Error al crear orden").await?;
                }
            }
        }

        Ok(())
    }

    async fn handle_order(&self, bot: Bot, msg: Message, order_request: OrderRequest) -> ResponseResult<()> {
        // Usar el exchange_manager directamente
        match self.exchange_manager.as_ref().place_order(
            &order_request.symbol,
            order_request.side,
            order_request.order_type,
            order_request.quantity,
            order_request.price,
        ).await {
            Ok(order) => {
                bot.send_message(
                    msg.chat.id,
                    format!(
                        "✅ Orden creada exitosamente!\n\
                         ID: {}\n\
                         Símbolo: {}\n\
                         Tipo: {:?}\n\
                         Cantidad: {}\n\
                         Precio: {}",
                        order.id, order.symbol, order.order_type,
                        order.quantity, order.price.unwrap_or_default()
                    )
                ).await?;
            }
            Err(e) => {
                error!("Error al crear orden: {}", e);
                bot.send_message(msg.chat.id, format!("❌ Error al crear orden: {}", e)).await?;
            }
        }

        Ok(())
    }

    // Similar para sell, balance, orders y cancel...
}

pub async fn handle_register(bot: Bot, msg: Message, db: Arc<Database>, text: String) -> ResponseResult<()> {
    tracing::info!("Iniciando registro para chat_id: {}", msg.chat.id.0);
    
    // Verificar si el usuario ya existe
    if let Ok(Some(existing_user)) = db.get_user_by_telegram_id(msg.chat.id.0).await {
        tracing::info!("Usuario ya existe: {:?}", existing_user);
        bot.send_message(
            msg.chat.id,
            "Ya estás registrado!"
        ).await?;
        return Ok(());
    }

    // Obtener el username del mensaje o usar el proporcionado en el comando
    let username = if !text.is_empty() {
        text
    } else {
        msg.from()
            .and_then(|user| user.username.clone())
            .unwrap_or_else(|| format!("user_{}", msg.chat.id.0))
    };

    tracing::info!("Creando nuevo usuario con username: {}", username);

    // Crear el usuario
    match db.create_telegram_user(msg.chat.id.0, &username).await {
        Ok(user_id) => {
            tracing::info!("Usuario creado exitosamente con ID: {}", user_id);
            bot.send_message(
                msg.chat.id,
                "✅ Registro exitoso!\nAhora puedes usar los comandos del bot."
            ).await?;
        }
        Err(e) => {
            tracing::error!("Error al crear usuario: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error al registrar usuario. Por favor intente nuevamente."
            ).await?;
        }
    }

    Ok(())
}

pub async fn handle_alert(bot: Bot, msg: Message, text: String, db: Arc<Database>) -> ResponseResult<()> {
    let args: Vec<&str> = text.split_whitespace().collect();
    
    if args.len() != 3 {
        bot.send_message(
            msg.chat.id,
            "Uso: /alert <symbol> <precio> <above|below>"
        ).await?;
        return Ok(());
    }

    // Primero verificar que el usuario existe
    match db.get_user_by_telegram_id(msg.chat.id.0).await {
        Ok(Some(user)) => {
            tracing::info!("Usuario encontrado: id={}, telegram_id={}", user.id, msg.chat.id.0);
            
            let price = match args[1].parse::<f64>() {
                Ok(p) => p,
                Err(_) => {
                    bot.send_message(msg.chat.id, "Precio inválido").await?;
                    return Ok(());
                }
            };

            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: args[0].to_uppercase(),
                target_price: price,
                condition: match args[2].to_lowercase().as_str() {
                    "above" => AlertCondition::Above,
                    "below" => AlertCondition::Below,
                    _ => {
                        bot.send_message(msg.chat.id, "Condición inválida. Use 'above' o 'below'").await?;
                        return Ok(());
                    }
                },
                created_at: chrono::Utc::now().timestamp(),
                triggered: false,
            };

            tracing::info!("Intentando crear alerta: {:?}", alert);

            match db.save_alert(alert).await {
                Ok(alert_id) => {
                    tracing::info!("Alerta creada con ID: {}", alert_id);
                    bot.send_message(
                        msg.chat.id,
                        format!("✅ Alerta creada exitosamente!\nID: {}", alert_id)
                    ).await?;
                }
                Err(e) => {
                    tracing::error!("Error al crear alerta en la base de datos: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "❌ Error al crear la alerta. Por favor intente nuevamente."
                    ).await?;
                }
            }
        }
        Ok(None) => {
            tracing::error!("Usuario no encontrado para telegram_id: {}", msg.chat.id.0);
            bot.send_message(
                msg.chat.id,
                "❌ Usuario no registrado. Use /register primero."
            ).await?;
        }
        Err(e) => {
            tracing::error!("Error al buscar usuario: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error interno. Por favor intente nuevamente."
            ).await?;
        }
    }

    Ok(())
}

pub async fn handle_depeg(bot: Bot, msg: Message, text: String) -> ResponseResult<()> {
    let args: Vec<&str> = text.trim().split_whitespace().collect();
    
    // Mostrar ayuda si no hay argumentos o se solicita explícitamente
    if args.is_empty() || args[0] == "help" || args.len() != 2 {
        let supported_stables = get_supported_stablecoins()
            .iter()
            .map(|s| format!("• `{}`", s))
            .collect::<Vec<_>>()
            .join("\n");

        let help_text = format!(
            "🔔 *Crear Alerta de Depeg*\n\n\
             *Uso:* `/depeg <stablecoin> <diferencial>`\n\n\
             *Ejemplo:*\n\
             • `/depeg USDT 0\\.02` \\- Alerta si USDT se desvía más de 2\\% de 1 USD\n\n\
             *Stablecoins Soportados:*\n\
             {}", supported_stables
        );

        bot.send_message(msg.chat.id, escape_markdown(&help_text))
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    let symbol = args[0].to_uppercase();
    let threshold = match args[1].parse::<f64>() {
        Ok(t) => t,
        Err(_) => {
            bot.send_message(
                msg.chat.id,
                "❌ El diferencial debe ser un número válido"
            ).await?;
            return Ok(());
        }
    };

    // Verificar que el stablecoin esté soportado
    if !CRYPTO_CONFIG.stablecoins.contains_key(&symbol) {
        bot.send_message(
            msg.chat.id,
            format!("❌ Stablecoin no soportado: {}\nUsa /depeg para ver la lista de stablecoins soportados", symbol)
        ).await?;
        return Ok(());
    }

    // Formatear mensaje de confirmación
    let confirmation = format!(
        "✅ Alerta de depeg creada:\n\
         Stablecoin: {}\n\
         Diferencial: {:.1}%",
        symbol,
        threshold * 100.0
    );

    bot.send_message(msg.chat.id, escape_markdown(&confirmation))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    
    Ok(())
}

pub async fn handle_pair_depeg(bot: Bot, msg: Message, text: String) -> ResponseResult<()> {
    let args: Vec<&str> = text.trim().split_whitespace().collect();
    
    // Mostrar ayuda si no hay argumentos o se solicita explícitamente
    if args.is_empty() || args[0] == "help" || args.len() != 3 {
        let supported_pairs = get_supported_pairs()
            .iter()
            .map(|(token1, token2)| format!("• `{}/{}`", token1, token2))
            .collect::<Vec<_>>()
            .join("\n");

        let help_text = format!(
            "🔔 *Crear Alerta de Par*\n\n\
             *Uso:* `/pairdepeg <token1> <token2> <diferencial>`\n\n\
             *Ejemplo:*\n\
             • `/pairdepeg USDT USDC 0\\.01` \\- Alerta si la diferencia entre USDT y USDC supera el 1\\%\n\n\
             *Pares Soportados:*\n\
             {}", supported_pairs
        );

        bot.send_message(msg.chat.id, escape_markdown(&help_text))
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
        return Ok(());
    }

    let token1 = args[0].to_uppercase();
    let token2 = args[1].to_uppercase();
    let threshold = match args[2].parse::<f64>() {
        Ok(t) => t,
        Err(_) => {
            bot.send_message(
                msg.chat.id,
                "❌ El diferencial debe ser un número válido entre 0 y 1\\. Ejemplo: 0\\.02 para 2\\%"
            )
            .parse_mode(ParseMode::MarkdownV2)
            .await?;
            return Ok(());
        }
    };

    // Verificar que el par esté soportado
    let pair_exists = CRYPTO_CONFIG.synthetic_pairs.values().any(|pair| {
        (pair.token1 == token1 && pair.token2 == token2) ||
        (pair.token1 == token2 && pair.token2 == token1)
    });

    if !pair_exists {
        bot.send_message(
            msg.chat.id,
            escape_markdown(&format!("❌ Par no soportado: {}/{}\nUsa /pairdepeg para ver la lista de pares soportados", token1, token2))
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    // Formatear mensaje de confirmación
    let confirmation = format!(
        "✅ Alerta de par creada:\n\
         Par: {}/{}\n\
         Diferencial: {:.1}%",
        token1, token2,
        threshold * 100.0
    );

    bot.send_message(msg.chat.id, escape_markdown(&confirmation))
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
    
    Ok(())
}

pub async fn handle_list_alerts(bot: Bot, msg: Message, db: Arc<Database>) -> ResponseResult<()> {
    match db.get_user_by_telegram_id(msg.chat.id.0).await {
        Ok(Some(user)) => {
            match db.get_user_alerts(user.id).await {
                Ok(alerts) => {
                    if alerts.is_empty() {
                        bot.send_message(
                            msg.chat.id,
                            "No tienes alertas activas."
                        ).await?;
                    } else {
                        let alerts_text = alerts.iter()
                            .map(|alert| {
                                format!(
                                    "🔔 *Alerta {}*\n\
                                     Símbolo: `{}`\n\
                                     Precio: `{}`\n\
                                     Condición: `{}`",
                                    alert.id.unwrap_or(0),
                                    escape_markdown(&alert.symbol),
                                    alert.target_price,
                                    escape_markdown(&alert.condition.to_string())
                                )
                            })
                            .collect::<Vec<_>>()
                            .join("\n\n");

                        bot.send_message(msg.chat.id, alerts_text)
                            .parse_mode(ParseMode::MarkdownV2)
                            .await?;
                    }
                }
                Err(e) => {
                    tracing::error!("Error al obtener alertas: {}", e);
                    bot.send_message(
                        msg.chat.id,
                        "❌ Error al obtener las alertas. Por favor intente nuevamente."
                    ).await?;
                }
            }
        }
        Ok(None) => {
            bot.send_message(
                msg.chat.id,
                "❌ Usuario no registrado. Use /register primero."
            ).await?;
        }
        Err(e) => {
            tracing::error!("Error al buscar usuario: {}", e);
            bot.send_message(
                msg.chat.id,
                "❌ Error interno. Por favor intente nuevamente."
            ).await?;
        }
    }

    Ok(())
}

// Función auxiliar para formatear alertas
fn format_alert(id: i64, description: &str) -> String {
    format!("{}\\. {}", id, description)
}

fn format_price_alert(id: i64, symbol: &str, price: f64, condition: &str) -> String {
    let symbol = escape_markdown(symbol);
    let price_str = format!("\\${:.2}", price)
        .replace(".", "\\.");  // Escapar el punto decimal
    let condition_symbol = if condition == "above" { "\\>" } else { "\\<" };
    
    format_alert(
        id,
        &format!("{} {} {}", 
            symbol,
            condition_symbol,
            price_str
        )
    )
}

fn format_depeg_alert(id: i64, symbol: &str, threshold: f64) -> String {
    let symbol = escape_markdown(symbol);
    let threshold_str = format!("{:.1}\\%", threshold * 100.0)
        .replace(".", "\\.");  // Escapar el punto decimal
    
    format_alert(
        id,
        &format!("{} depeg \\> {}", 
            symbol,
            threshold_str
        )
    )
}

fn format_pair_alert(id: i64, token1: &str, token2: &str, threshold: f64) -> String {
    let token1 = escape_markdown(token1);
    let token2 = escape_markdown(token2);
    let threshold_str = format!("{:.1}\\%", threshold * 100.0)
        .replace(".", "\\.");  // Escapar el punto decimal
    
    format_alert(
        id,
        &format!("{}/{} depeg \\> {}", 
            token1, token2,
            threshold_str
        )
    )
}

pub async fn handle_delete_alert(bot: Bot, msg: Message, text: String) -> ResponseResult<()> {
    let args: Vec<&str> = text.split_whitespace().collect();
    
    if args.is_empty() || args[0] == "help" {
        bot.send_message(
            msg.chat.id,
            "*Comando /delete \\- Eliminar una alerta*\n\n\
             Uso: `/delete <id_alerta>`\n\n\
             Usa /alerts para ver los IDs de tus alertas"
        )
        .parse_mode(ParseMode::MarkdownV2)
        .await?;
        return Ok(());
    }

    // TODO: Implementar la lógica de eliminación
    bot.send_message(
        msg.chat.id,
        "🚧 Eliminación de alertas en desarrollo..."
    ).await?;
    Ok(())
} 