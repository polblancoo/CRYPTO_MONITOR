use std::env;
use std::error::Error;
use dotenv::dotenv;

pub mod crypto_config;
pub use self::crypto_config::*;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub database_url: String,
    pub coingecko_api_key: String,
    pub telegram_token: String,
    pub check_interval: u64,
}

impl AppConfig {
    pub fn new() -> Result<Self, Box<dyn Error + Send + Sync>> {
        dotenv().ok();

        Ok(Self {
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite::memory:".to_string()),
            coingecko_api_key: env::var("COINGECKO_API_KEY")?,
            telegram_token: env::var("TELEGRAM_BOT_TOKEN")?,
            check_interval: env::var("CHECK_INTERVAL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
        })
    }
} 