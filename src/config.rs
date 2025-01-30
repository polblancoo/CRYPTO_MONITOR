use std::env;
use serde::Deserialize;
use once_cell::sync::Lazy;

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

#[derive(Debug, Deserialize)]
pub struct CryptoConfig {
    pub cryptocurrencies: Vec<String>,
    pub stablecoins: Vec<String>,
    pub synthetic_pairs: Vec<(String, String)>,
    pub exchanges: Vec<String>,
    pub check_interval: u64,
}

#[derive(Debug, Deserialize)]
pub struct CryptoInfo {
    pub name: String,
    pub coingecko_id: String,
}

#[derive(Debug, Deserialize)]
pub struct StablecoinInfo {
    pub name: String,
    pub target_price: f64,
}

#[derive(Debug, Deserialize)]
pub struct PairInfo {
    pub token1: String,
    pub token2: String,
    pub expected_ratio: f64,
}

#[derive(Debug, Deserialize)]
pub struct ExchangeConfig {
    pub supported: Vec<String>,
}

pub static CONFIG: Lazy<CryptoConfig> = Lazy::new(|| {
    let check_interval = env::var("CHECK_INTERVAL")
        .unwrap_or_else(|_| "300".to_string())
        .parse()
        .unwrap_or(300);

    CryptoConfig {
        cryptocurrencies: vec![
            "BTC".to_string(),
            "ETH".to_string(),
            "RUNE".to_string(),
            "MATIC".to_string(),
            "SOL".to_string(),
        ],
        stablecoins: vec![
            "USDT".to_string(),
            "USDC".to_string(),
            "BUSD".to_string(),
        ],
        synthetic_pairs: vec![
            ("BTC".to_string(), "USDT".to_string()),
            ("ETH".to_string(), "USDT".to_string()),
            ("RUNE".to_string(), "USDT".to_string()),
        ],
        exchanges: vec![
            "binance".to_string(),
        ],
        check_interval,
    }
});

impl CryptoConfig {
    pub fn get_symbol_display(&self, symbol: &str) -> String {
        if self.cryptocurrencies.contains(&symbol.to_string()) {
            symbol.to_string()
        } else {
            symbol.to_string()
        }
    }

    pub fn get_supported_symbols(&self) -> Vec<String> {
        self.cryptocurrencies.clone()
    }

    pub fn get_supported_pairs(&self) -> Vec<(String, String)> {
        self.synthetic_pairs.clone()
    }

    pub fn get_stablecoins(&self) -> Vec<String> {
        self.stablecoins.clone()
    }

    pub fn is_supported_exchange(&self, exchange: &str) -> bool {
        self.exchanges.contains(&exchange.to_string())
    }

    pub fn get_check_interval(&self) -> u64 {
        self.check_interval
    }
} 