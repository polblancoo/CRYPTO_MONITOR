use crypto_monitor::{
    Config, Database,
    start_monitor,
    bot::TelegramBot,
};
use dotenv::dotenv;
use tracing::{info, error};
use tracing_subscriber::FmtSubscriber;
use std::sync::Arc;
use tokio::join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    dotenv().ok();
    
    // Configurar logging
    tracing_subscriber::fmt()
        .with_env_filter("crypto_monitor=debug")
        .init();
    
    info!("Iniciando Crypto Monitor...");
    
    let config = Config::new()?;
    let db = Arc::new(Database::new(&config.database_url)?);
    
    // Verificar token de Telegram
    TelegramBot::verify_bot_token().await?;
    
    // Crear y ejecutar el bot en un task separado
    let bot = TelegramBot::new(db.clone());
    let bot_handle = tokio::spawn(async move {
        info!("Iniciando bot de Telegram...");
        bot.run().await;
    });

    // Iniciar el monitor de precios en el task principal
    info!("Iniciando monitor de precios...");
    let monitor_handle = tokio::spawn(async move {
        if let Err(e) = start_monitor(&config, db).await {
            error!("Error en el monitor: {}", e);
        }
    });

    // Esperar a que ambos servicios terminen (o manejar errores)
    tokio::select! {
        result = bot_handle => {
            if let Err(e) = result {
                error!("Error en el bot de Telegram: {}", e);
            }
        }
        result = monitor_handle => {
            if let Err(e) = result {
                error!("Error en el monitor de precios: {}", e);
            }
        }
    }
    
    Ok(())
} 