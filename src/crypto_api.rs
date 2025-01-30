use crate::models::CryptoPrice;
use reqwest::Client;
use std::{error::Error, time::Duration, collections::HashMap};
use serde::Deserialize;
use tokio::time::sleep;
use tracing::{info, error};
use crate::config::CONFIG;

pub struct CryptoAPI {
    client: Client,
    api_key: String,
    symbol_to_id: HashMap<String, String>,
    supported_exchanges: Vec<String>,
}

#[derive(Deserialize)]
struct CoinGeckoResponse {
    #[serde(flatten)]
    prices: HashMap<String, ExchangePrices>,
}

#[derive(Deserialize)]
struct ExchangePrices {
    #[serde(rename = "usd")]
    price: f64,
    #[serde(rename = "usd_24h_vol")]
    volume_24h: Option<f64>,
    #[serde(rename = "usd_market_cap")]
    market_cap: Option<f64>,
}

#[derive(Debug)]
pub struct ExchangePrice {
    pub exchange: String,
    pub price: f64,
    pub volume_24h: Option<f64>,
}

#[derive(Debug, Clone)]
pub struct CoinInfo {
    pub symbol: String,
    pub name: String,
    pub supported_exchanges: Vec<String>,
}

impl CryptoAPI {
    pub fn new(api_key: String) -> Self {
        Self {
            client: Client::new(),
            api_key,
            symbol_to_id: HashMap::new(), // Simplificado por ahora
            supported_exchanges: CONFIG.exchanges.clone(),
        }
    }

    pub async fn get_price(&self, symbol: &str) -> Result<CryptoPrice, Box<dyn Error + Send + Sync>> {
        // Obtener precio del exchange por defecto (binance)
        self.get_price_from_exchange(symbol, "binance").await
    }

    pub async fn get_price_from_exchange(&self, symbol: &str, exchange: &str) -> Result<CryptoPrice, Box<dyn Error + Send + Sync>> {
        const MAX_RETRIES: u32 = 3;
        const RETRY_DELAY: u64 = 5;

        if !self.supported_exchanges.contains(&exchange.to_string()) {
            return Err(format!("Exchange no soportado: {}", exchange).into());
        }

        let coin_id = self.symbol_to_id
            .get(&symbol.to_uppercase())
            .ok_or_else(|| format!("Símbolo no soportado: {}", symbol))?;
        
        for attempt in 0..MAX_RETRIES {
            if attempt > 0 {
                info!("Reintento {} de obtener precio para {} en {}", attempt + 1, symbol, exchange);
                sleep(Duration::from_secs(RETRY_DELAY)).await;
            }

            let url = format!(
                "https://api.coingecko.com/api/v3/simple/price?ids={}&vs_currencies=usd&x_cg_demo_api_key={}&include_24hr_vol=true&include_market_cap=true&include_exchange_logo=true&exchanges={}",
                coin_id,
                self.api_key,
                exchange
            );
            
            info!("Consultando precio de {} en {}", symbol, exchange);

            match self.client
                .get(&url)
                .timeout(Duration::from_secs(10))
                .send()
                .await
            {
                Ok(response) => {
                    info!("Respuesta recibida de {}: Status {}", exchange, response.status());
                    if response.status().is_success() {
                        match response.json::<CoinGeckoResponse>().await {
                            Ok(data) => {
                                if let Some(prices) = data.prices.get(coin_id) {
                                    info!("Precio de {} en {}: ${}", symbol, exchange, prices.price);
                                    return Ok(CryptoPrice {
                                        symbol: symbol.to_uppercase(),
                                        price: prices.price,
                                        exchange: exchange.to_string(),
                                        timestamp: chrono::Utc::now().timestamp(),
                                    });
                                }
                            }
                            Err(e) => error!("Error al deserializar respuesta de {}: {}", exchange, e),
                        }
                    } else {
                        let status = response.status();
                        let error_text = response.text().await.unwrap_or_default();
                        error!("Error de API en {}: {} - {}", exchange, status, error_text);
                    }
                }
                Err(e) => error!("Error de conexión con {}: {}", exchange, e),
            }
        }

        Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::Other,
            format!("No se pudo obtener el precio para {} en {} después de {} intentos", 
                   symbol, exchange, MAX_RETRIES)
        )))
    }

    pub async fn get_prices_from_all_exchanges(&self, symbol: &str) -> Vec<ExchangePrice> {
        let mut prices = Vec::new();
        
        for exchange in &self.supported_exchanges {
            match self.get_price_from_exchange(symbol, exchange).await {
                Ok(price) => {
                    prices.push(ExchangePrice {
                        exchange: exchange.clone(),
                        price: price.price,
                        volume_24h: None, // TODO: Implementar volumen
                    });
                }
                Err(e) => {
                    error!("Error al obtener precio de {} en {}: {}", symbol, exchange, e);
                }
            }
        }

        prices
    }

    pub fn supported_symbols(&self) -> Vec<String> {
        self.symbol_to_id.keys().cloned().collect()
    }

    pub fn supported_exchanges(&self) -> Vec<String> {
        self.supported_exchanges.clone()
    }
}

pub async fn get_supported_coins() -> Result<Vec<CoinInfo>, Box<dyn Error>> {
    let mut coins = Vec::new();
    
    for symbol in &CONFIG.cryptocurrencies {
        coins.push(CoinInfo {
            symbol: symbol.clone(),
            name: symbol.clone(),
            supported_exchanges: CONFIG.exchanges.clone(),
        });
    }
    
    Ok(coins)
}

pub async fn get_coin_price(_symbol: &str) -> Result<f64, Box<dyn Error>> {
    // Implementar lógica real de precio aquí
    Ok(0.0)
}

pub async fn get_stablecoin_price(_symbol: &str) -> Result<f64, Box<dyn Error>> {
    // Implementar lógica real de precio aquí
    Ok(1.0)
}

pub async fn get_pair_ratio(token1: &str, token2: &str) -> Result<f64, Box<dyn Error>> {
    let price1 = get_coin_price(token1).await?;
    let price2 = get_coin_price(token2).await?;
    
    if price2 == 0.0 {
        return Err("División por cero".into());
    }
    
    Ok(price1 / price2)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_price_from_exchange() {
        let api = CryptoAPI::new("demo-key".to_string());
        let result = api.get_price_from_exchange("BTC", "binance").await;
        assert!(result.is_ok(), "Debería poder obtener el precio de Bitcoin en Binance");
        
        let price = result.unwrap();
        assert_eq!(price.symbol, "BTC");
        assert!(price.price > 0.0, "El precio debería ser mayor que 0");
        assert_eq!(price.exchange, "binance");
    }

    #[tokio::test]
    async fn test_get_prices_from_all_exchanges() {
        let api = CryptoAPI::new("demo-key".to_string());
        let prices = api.get_prices_from_all_exchanges("BTC").await;
        assert!(!prices.is_empty(), "Debería obtener precios de al menos un exchange");
    }

    #[test]
    fn test_supported_exchanges() {
        let api = CryptoAPI::new("demo-key".to_string());
        let exchanges = api.supported_exchanges();
        assert!(exchanges.contains(&"binance".to_string()));
        assert!(exchanges.contains(&"coinbase".to_string()));
    }
} 