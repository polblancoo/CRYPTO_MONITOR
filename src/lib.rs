pub mod api;
pub mod auth;
pub mod crypto_api;
pub mod db;
pub mod models;
pub mod monitor;
pub mod notify;
pub mod timer;

pub use crate::auth::Auth;
pub use crate::db::Database;
pub use crate::models::*;
pub use crate::crypto_api::CryptoAPI;

use dotenv::dotenv;
use std::env;
use std::error::Error;
use std::sync::Arc;

pub struct Config {
    pub database_url: String,
    pub coingecko_api_key: String,
    pub telegram_token: String,
    pub check_interval: u64,
}

impl Config {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        dotenv().ok();

        Ok(Self {
            database_url: env::var("DATABASE_URL")?,
            coingecko_api_key: env::var("COINGECKO_API_KEY")?,
            telegram_token: env::var("TELEGRAM_BOT_TOKEN")?,
            check_interval: env::var("CHECK_INTERVAL")?.parse()?,
        })
    }
}

pub async fn start_monitor(config: &Config, db: Arc<Database>) -> Result<(), Box<dyn Error>> {
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