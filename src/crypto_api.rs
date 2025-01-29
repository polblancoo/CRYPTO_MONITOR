use crate::models::CryptoPrice;
use reqwest::Client;
use std::{error::Error, time::Duration};
use serde::Deserialize;
use std::collections::HashMap;
use tokio::time::sleep;
use tracing::{info, error};

pub struct CryptoAPI {
    client: Client,
    api_key: String,
    symbol_to_id: HashMap<String, String>,
}

#[derive(Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    prices: std::collections::HashMap<String, Price>,
}

#[derive(Deserialize)]
struct Price {
    usd: f64,
}

impl CryptoAPI {
    pub fn new(api_key: String) -> Self {
        let mut symbol_to_id = HashMap::new();
        symbol_to_id.insert("BTC".to_string(), "bitcoin".to_string());
        symbol_to_id.insert("ETH".to_string(), "ethereum".to_string());
        symbol_to_id.insert("USDT".to_string(), "tether".to_string());
        symbol_to_id.insert("BNB".to_string(), "binancecoin".to_string());
        symbol_to_id.insert("SOL".to_string(), "solana".to_string());
        symbol_to_id.insert("XRP".to_string(), "ripple".to_string());
        symbol_to_id.insert("USDC".to_string(), "usd-coin".to_string());
        symbol_to_id.insert("ADA".to_string(), "cardano".to_string());
        symbol_to_id.insert("AVAX".to_string(), "avalanche-2".to_string());
        symbol_to_id.insert("DOGE".to_string(), "dogecoin".to_string());

        Self {
            client: Client::new(),
            api_key,
            symbol_to_id,
        }
    }

    pub async fn get_price(&self, symbol: &str) -> Result<CryptoPrice, Box<dyn Error + Send + Sync>> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: u64 = 5;

        let coin_id = self.symbol_to_id
            .get(&symbol.to_uppercase())
            .ok_or_else(|| format!("Símbolo no soportado: {}", symbol))?;
        
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                info!("Reintento {} de obtener precio para {}", attempt + 1, symbol);
                sleep(Duration::from_secs(RETRY_DELAY)).await;
            }

            let url = format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&x_cg_demo_api_key={}",
                coin_id,
                self.api_key
            );
            
            info!("Intentando obtener precio de {}, URL: {}", symbol, url);

            match self.client
                .get(&url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    info!("Respuesta recibida: Status {}", response.status());
                    if response.status().is_success() {
                        match response.json::<CoinGeckoResponse>().await {
                            Ok(data) => {
                                if let Some(price) = data.prices.get(coin_id) {
                                    info!("Precio obtenido para {}: ${}", symbol, price.usd);
                                    return Ok(CryptoPrice {
                                        symbol: symbol.to_uppercase(),
                                        price: price.usd,
                                        exchange: "coingecko".to_string(),
                                        timestamp: chrono::Utc::now().timestamp(),
                                    });
                                }
                            }
                            Err(e) => error!("Error al deserializar respuesta: {}", e),
                        }
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();
                        error!("Error de API: {} - {}", status, error_text);
                    }
                }
                Err(e) => error!("Error de conexión: {}", e),
            }
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No se pudo obtener el precio para {} después de {} intentos", symbol, MAX_RETRIES)
        )))
    }

    pub fn supported_symbols(&self) -> Vec<String> {
        self.symbol_to_id.keys().cloned().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price() {
        let api = CryptoAPI::new("demo-key".to_string());
        let result = api.get_price("BTC").await;
        assert!(result.is_ok(), "Debería poder obtener el precio de Bitcoin");
        
        let price = result.unwrap();
        assert_eq!(price.symbol, "BTC");
        assert!(price.price > 0.0, "El precio debería ser mayor que 0");
    }

    #[test]
    fn test_supported_symbols() {
        let api = CryptoAPI::new("demo-key".to_string());
        let symbols = api.supported_symbols();
        assert!(symbols.contains(&"BTC".to_string()));
        assert!(symbols.contains(&"ETH".to_string()));
    }
} 