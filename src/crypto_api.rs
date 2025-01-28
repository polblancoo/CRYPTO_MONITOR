use crate::models::CryptoPrice;
use reqwest::Client;
use std::error::Error;

pub struct CryptoAPI {
    client: Client,
    api_key: String,
}

impl CryptoAPI {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
        }
    }

    pub async fn get_price(&self, _symbol: &str) -> Result<CryptoPrice, Box<dyn Error>> {
        // Aquí implementaremos la llamada a CoinGecko
        todo!("Implementar llamada a API")
    }
} 