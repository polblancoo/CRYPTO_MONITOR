pub mod api;
pub mod auth;
pub mod crypto_api;
pub mod db;
pub mod models;
pub mod monitor;
pub mod notify;
pub mod timer;
pub mod bot;
pub mod exchanges;
pub mod config;

pub use auth::Auth;
pub use db::Database;
pub use models::*;
pub use crypto_api::CryptoAPI;
pub use monitor::PriceMonitor;
pub use notify::NotificationService;
pub use config::AppConfig;
pub use config::crypto_config::*;

// Re-exportar AlertCondition
pub use models::AlertCondition;

use std::sync::Arc;
use crate::exchanges::ExchangeManager;

pub async fn start_monitor(config: &AppConfig, db: Arc<Database>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let api = CryptoAPI::new(config.coingecko_api_key.clone());
    let notification_service = NotificationService::new(config.telegram_token.clone());
    let monitor = PriceMonitor::new(
        api,
        notification_service,
        db,
        config.check_interval,
    );

    monitor.start().await
}

pub async fn start_api(_port: u16, db: Arc<Database>) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let exchange_manager = Arc::new(ExchangeManager::new()?);
    api::start_server(db, exchange_manager).await?;
    Ok(())
}

pub async fn run() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let db = Arc::new(Database::new("crypto.db").await?);
    let exchange_manager = Arc::new(ExchangeManager::new()?);
    
    api::start_server(
        Arc::clone(&db),
        Arc::clone(&exchange_manager)
    ).await?;

    Ok(())
} 