use std::sync::Arc;
use tokio::{self, signal, task::JoinSet, sync::broadcast};
use dotenv::dotenv;
use crypto_monitor::{
    db::Database,
    bot::TelegramBot,
    monitor::PriceMonitor,
    exchanges::{ExchangeManager, ExchangeError},
    crypto_api::CryptoAPI,
    notify::NotificationService,
    config::AppConfig,
};
use teloxide::Bot;
use tracing::{info, error};
use tracing_subscriber::{fmt, EnvFilter};
use std::time::Duration;
use std::time::Instant;
use tokio::time::sleep;
use tokio::select;

#[tokio::main(worker_threads = 4)]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    // Cargar variables de entorno
    dotenv().ok();
    
    // Inicializar logging
    fmt()
        .with_env_filter(EnvFilter::from_default_env()
            .add_directive(tracing::Level::INFO.into()))
        .with_target(false)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .init();

    // Inicializar componentes
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "sqlite:crypto_monitor.db".to_string());
    
    let db = Arc::new(Database::new(&database_url).await?);
    
    // Obtener API key de CoinGecko
    let coingecko_api_key = std::env::var("COINGECKO_API_KEY")
        .expect("COINGECKO_API_KEY debe estar configurado");
    
    let exchange_manager = Arc::new(ExchangeManager::new()?);
    let crypto_api = CryptoAPI::new(coingecko_api_key);
    
    let bot_token = std::env::var("TELEGRAM_BOT_TOKEN")
        .expect("TELEGRAM_BOT_TOKEN debe estar configurado");
    
    let notification_service = NotificationService::new(bot_token.clone());
    
    // Crear monitor de precios
    let monitor = Arc::new(PriceMonitor::new(
        crypto_api,
        notification_service,
        db.clone(),
        60, // intervalo en segundos
    ));

    // Crear bot de Telegram
    let bot = Arc::new(TelegramBot::new(
        bot_token,
        db.clone(),
        exchange_manager.clone(),
    ));

    // Ejecutar bot y monitor en paralelo
    let monitor_handle = {
        let monitor = monitor.clone();
        tokio::spawn(async move {
            if let Err(e) = monitor.start().await {
                error!("Error en el monitor: {}", e);
            }
        })
    };

    let bot_handle = {
        let bot = bot.clone();
        tokio::spawn(async move {
            if let Err(e) = bot.start().await {
                error!("Error en el bot: {}", e);
            }
        })
    };

    // Esperar a que ambos terminen o manejar errores
    select! {
        result = monitor_handle => {
            match result {
                Ok(_) => info!("Monitor terminado correctamente"),
                Err(e) => error!("Error en el monitor: {}", e),
            }
        }
        result = bot_handle => {
            match result {
                Ok(_) => info!("Bot terminado correctamente"),
                Err(e) => error!("Error en el bot: {}", e),
            }
        }
    }

    Ok(())
} 