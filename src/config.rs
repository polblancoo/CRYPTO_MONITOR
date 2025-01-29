use std::env;
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
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
    pub cryptocurrencies: HashMap<String, CryptoInfo>,
    pub stablecoins: HashMap<String, StablecoinInfo>,
    pub synthetic_pairs: HashMap<String, PairInfo>,
    pub exchanges: ExchangeConfig,
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
    let config_path = Path::new("config/crypto_config.toml");
    let config_str = fs::read_to_string(config_path)
        .expect("Failed to read crypto_config.toml");
    toml::from_str(&config_str)
        .expect("Failed to parse crypto_config.toml")
});

impl CryptoConfig {
    pub fn get_symbol_display(&self, symbol: &str) -> String {
        if let Some(info) = self.cryptocurrencies.get(symbol) {
            format!("{} ({})", symbol, info.name)
        } else {
            symbol.to_string()
        }
    }

    pub fn get_supported_symbols(&self) -> Vec<String> {
        self.cryptocurrencies.keys()
            .map(|s| s.to_string())
            .collect()
    }

    pub fn get_supported_pairs(&self) -> Vec<(String, String)> {
        self.synthetic_pairs.values()
            .map(|pair| (pair.token1.clone(), pair.token2.clone()))
            .collect()
    }

    pub fn get_stablecoins(&self) -> Vec<String> {
        self.stablecoins.keys()
            .map(|s| s.to_string())
            .collect()
    }
} 