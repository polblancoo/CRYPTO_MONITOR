use teloxide::{
    payloads::SendMessageSetters,
    prelude::*,
    types::{Message, ParseMode},
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
};
use rust_decimal::Decimal;
use std::{sync::Arc, str::FromStr};
use tracing::{error, info};

pub type HandlerResult = Result<(), Box<dyn std::error::Error + Send + Sync>>;

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
         `/help` \\- Ver este menú\n\
         `/start` \\- Iniciar el bot\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         💹 *TRADING EN BINANCE*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         `/balance` \\- Ver balance de la cuenta\n\
         `/order` \\- Crear orden de trading\n\
         `/orders` \\- Ver órdenes abiertas\n\
         `/cancel` \\- Cancelar una orden\n\
         `/symbols` \\- Ver pares disponibles\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         ⚠️ *SISTEMA DE ALERTAS*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         `/alert` \\- Crear alerta de precio\n\
         `/depeg` \\- Alerta de depeg stablecoin\n\
         `/pairdepeg` \\- Alerta de par\n\
         `/alerts` \\- Ver alertas activas\n\
         `/delete` \\- Eliminar una alerta\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         🔐 *CONFIGURACIÓN*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         `/register` \\- Registrar usuario\n\
         \n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         ℹ️ *INFORMACIÓN*\n\
         ▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬▬\n\
         • Para más detalles sobre un comando,\n\
           usa: `<comando> help`\n\
           Ejemplo: `/order help`\n\
         \n\
         • Pares soportados: `/symbols`\n\
         • Estado: Monitoreando {} pares\n\
         • Intervalo: {} segundos",
        crate::exchanges::get_all_pairs().len(),
        crate::config::CONFIG.check_interval
    );

    bot.send_message(msg.chat.id, formatted_text)
        .parse_mode(ParseMode::MarkdownV2)
        .await?;

    Ok(())
}

pub async fn handle_balance(bot: Bot, msg: Message, exchange_manager: Arc<ExchangeManager>) -> ResponseResult<()> {
    let response = match exchange_manager.get_balances().await {
        Ok(balances) => {
            let mut message = String::from("*Balances*\\n\\n");
            for balance in balances {
                message.push_str(&format!(
                    "Asset: `{}`\\n  Free: `{}`\\n  Locked: `{}`\\n  Total: `{}`\\n\\n",
                    balance.asset,
                    balance.free,
                    balance.locked,
                    balance.free + balance.locked
                ));
            }
            message
        },
        Err(e) => format!("❌ Error al obtener balances: {}", e),
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

    match exchange_manager.execute_order(ExchangeType::Binance, order_request).await {
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
    info!("Obteniendo órdenes abiertas...");
    
    match exchange_manager.get_open_orders(ExchangeType::Binance).await {
        Ok(orders) => {
            if orders.is_empty() {
                bot.send_message(msg.chat.id, "No hay órdenes abiertas").await?;
                return Ok(());
            }

            let mut message = String::from("*Órdenes Abiertas*\\n\\n");
            for order in orders {
                message.push_str(&format!(
                    "ID: `{}`\\n\
                     Symbol: `{}`\\n\
                     Side: `{:?}`\\n\
                     Type: `{:?}`\\n\
                     Quantity: `{}`\\n\
                     Price: `{}`\\n\
                     Status: `{:?}`\\n\\n",
                    order.id, order.symbol, order.side, order.order_type,
                    order.quantity, order.price.unwrap_or_default(), order.status
                ));
            }

            bot.send_message(msg.chat.id, message)
                .parse_mode(ParseMode::MarkdownV2)
                .await?;
        }
        Err(e) => {
            error!("Error al obtener órdenes: {}", e);
            bot.send_message(
                msg.chat.id,
                format!("❌ Error al obtener órdenes: {}", e)
            ).await?;
        }
    }

    Ok(())
}

pub async fn handle_cancel(bot: Bot, msg: Message, order_id: String, exchange_manager: Arc<ExchangeManager>) -> ResponseResult<()> {
    match exchange_manager.cancel_order(ExchangeType::Binance, &order_id).await {
        Ok(_) => {
            bot.send_message(
                msg.chat.id,
                format!("✅ Orden {} cancelada", order_id)
            ).await?;
        }
        Err(e) => {
            bot.send_message(
                msg.chat.id,
                format!("❌ Error al cancelar orden: {}", e)
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
) -> Result<(), RequestError> {
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
    args: String,
) -> ResponseResult<()> {
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

    match db.get_user_by_telegram_id(msg.chat.id.0).await {
        Ok(Some(_user)) => {
            match exchange_manager.execute_order(ExchangeType::Binance, order_request).await {
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
            match self.exchange_manager.execute_order(ExchangeType::Binance, order_request).await {
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
        match self.exchange_manager.execute_order(ExchangeType::Binance, order_request).await {
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