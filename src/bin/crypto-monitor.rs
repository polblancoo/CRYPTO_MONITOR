use crypto_monitor::{
    Config, Database, models::*,
    crypto_api::CryptoAPI,
    auth::Auth,
    monitor::PriceMonitor,
    notify::NotificationService,
    timer::Timer,
    start_monitor,
};
use dotenv::dotenv;
use tracing_subscriber;
use std::sync::Arc;
use tokio::join;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenv().ok();
    
    // Configurar logging
    tracing_subscriber::fmt::init();
    
    println!("Iniciando monitor de criptomonedas...");
    
    let config = Config::new()?;
    let db = Arc::new(Database::new(&config.database_url)?);
    let auth = Auth::new(&db);
    let api = CryptoAPI::new(config.coingecko_api_key);
    let notification_service = NotificationService::new(config.telegram_token);
    let monitor = PriceMonitor::new(api, notification_service);
    
    // Iniciar el servidor API y el monitor en paralelo
    let server_handle = crypto_monitor::api::start_server(db.clone(), 3000);
    let monitor_handle = start_monitor(&config, db);
    
    // Esperar a que ambos terminen (aunque en realidad nunca deberían terminar)
    let _ = join!(server_handle, monitor_handle);
    
    Ok(())
} 