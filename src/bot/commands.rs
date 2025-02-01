use teloxide::{
    utils::command::BotCommands, 
    types::BotCommand,
    prelude::Requester,
};
use tokio::time::{sleep, Duration};
use std::time::Duration as StdDuration;

#[derive(BotCommands, Clone)]
#[command(rename_rule = "lowercase", description = "Comandos disponibles:")]
pub enum Command {
    #[command(description = "📝 Mostrar este mensaje de ayuda")]
    Help,
    #[command(description = "🚀 Iniciar el bot")]
    Start,
    #[command(description = "📋 Registrarse en el bot")]
    Register { text: String },
    #[command(description = "🔔 Crear alerta de precio - /alert BTCUSDT 40000 above")]
    Alert { text: String },
    #[command(description = "⚠️ Crear alerta de depeg - /depeg USDT 0\\.02")]
    Depeg { text: String },
    #[command(description = "🔄 Crear alerta de par - /pairdepeg USDT USDC 0\\.01")]
    PairDepeg { text: String },
    #[command(description = "📊 Ver tus alertas activas")]
    Alerts,
    #[command(description = "❌ Eliminar una alerta - /delete <id>")]
    Delete { text: String },
    #[command(description = "💱 Ver pares de trading disponibles")]
    Symbols,
    #[command(description = "💰 Ver balance de la cuenta")]
    Balance { text: String },
    #[command(description = "🔗 Conectar exchange - /connect binance <api_key> <api_secret>")]
    Connect { text: String },
    #[command(description = "📈 Crear orden de compra - /buy BTCUSDT market 0\\.001")]
    Buy { text: String },
    #[command(description = "📉 Crear orden de venta - /sell BTCUSDT limit 0\\.001 40000")]
    Sell { text: String },
    #[command(description = "📋 Ver órdenes abiertas")]
    Orders { text: String },
    #[command(description = "🔍 Ver detalles de una orden - /order <id>")]
    Order(String),
    #[command(description = "🚫 Cancelar órdenes - Muestra lista de órdenes activas para cancelar")]
    Cancel { text: String },
}

impl Command {
    pub fn set_my_commands(bot: teloxide::Bot) {
        tokio::spawn(async move {
            let commands = vec![
                BotCommand::new("help", "📝 Mostrar ayuda"),
                BotCommand::new("start", "🚀 Iniciar el bot"),
                BotCommand::new("register", "📋 Registrarse"),
                BotCommand::new("alert", "🔔 Crear alerta de precio"),
                BotCommand::new("depeg", "⚠️ Crear alerta de depeg"),
                BotCommand::new("pairdepeg", "🔄 Crear alerta de par"),
                BotCommand::new("alerts", "📊 Ver alertas activas"),
                BotCommand::new("delete", "❌ Eliminar alerta"),
                BotCommand::new("symbols", "💱 Ver pares disponibles"),
                BotCommand::new("balance", "💰 Ver balance"),
                BotCommand::new("connect", "🔗 Conectar exchange"),
                BotCommand::new("buy", "📈 Crear orden de compra"),
                BotCommand::new("sell", "📉 Crear orden de venta"),
                BotCommand::new("orders", "📋 Ver órdenes abiertas"),
                BotCommand::new("order", "🔍 Ver detalles de orden"),
                BotCommand::new("cancel", "🚫 Cancelar orden"),
            ];

            let mut retry_count = 0;
            let max_retries = 5;
            let initial_delay = StdDuration::from_secs(1);

            loop {
                match bot.set_my_commands(commands.clone()).await {
                    Ok(_) => {
                        tracing::info!("Comandos del bot configurados correctamente");
                        break;
                    }
                    Err(e) => {
                        retry_count += 1;
                        if retry_count > max_retries {
                            tracing::error!("Error fatal al configurar comandos del bot después de {} intentos: {}", max_retries, e);
                            break;
                        }
                        
                        let delay = initial_delay.mul_f32(1.5f32.powi(retry_count as i32));
                        tracing::warn!(
                            "Error al configurar comandos del bot (intento {}/{}): {}. Reintentando en {:?}...",
                            retry_count,
                            max_retries,
                            e,
                            delay
                        );
                        
                        sleep(Duration::from(delay)).await;
                    }
                }
            }
        });
    }
} 