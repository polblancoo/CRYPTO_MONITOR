use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use once_cell::sync::Lazy;

#[derive(Debug, Deserialize)]
pub struct CryptoConfig {
    pub cryptocurrencies: HashMap<String, CryptoInfo>,
    pub stablecoins: HashMap<String, StablecoinInfo>,
    pub synthetic_pairs: HashMap<String, PairInfo>,
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

pub static CRYPTO_CONFIG: Lazy<CryptoConfig> = Lazy::new(|| {
    let config_str = fs::read_to_string("config/crypto_config.toml")
        .expect("Error al leer crypto_config.toml");
    toml::from_str(&config_str).expect("Error al parsear crypto_config.toml")
});

pub fn get_supported_tokens() -> Vec<String> {
    CRYPTO_CONFIG.cryptocurrencies.keys().cloned().collect()
}

pub fn get_supported_stablecoins() -> Vec<String> {
    CRYPTO_CONFIG.stablecoins
        .keys()
        .cloned()
        .collect()
}

pub fn get_supported_pairs() -> Vec<(String, String)> {
    CRYPTO_CONFIG.synthetic_pairs
        .values()
        .map(|pair| (pair.token1.clone(), pair.token2.clone()))
        .collect()
} 