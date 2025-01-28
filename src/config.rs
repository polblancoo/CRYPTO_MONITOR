use std::env;

pub struct Config {
    pub telegram_token: String,
    pub coingecko_api_key: String,
    pub check_interval: u64,
    pub database_url: String,
}

impl Config {
    pub fn new() -> Result<Self, env::VarError> {
        Ok(Config {
            telegram_token: env::var("TELEGRAM_BOT_TOKEN")?,
            coingecko_api_key: env::var("COINGECKO_API_KEY")?,
            check_interval: env::var("CHECK_INTERVAL")
                .unwrap_or_else(|_| "300".to_string())
                .parse()
                .unwrap_or(300),
            database_url: env::var("DATABASE_URL")
                .unwrap_or_else(|_| "sqlite::memory:".to_string()),
        })
    }
} 