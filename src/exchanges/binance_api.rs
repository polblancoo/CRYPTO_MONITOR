use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::error::Error;
use tracing::{info, error};
use rust_decimal::Decimal;
use std::str::FromStr;
use super::errors::ExchangeError;

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub balances: Vec<Balance>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

pub struct BinanceApi {
    client: reqwest::Client,
    api_key: String,
    api_secret: String,
}

impl BinanceApi {
    pub fn new(api_key: String, api_secret: String) -> Self {
        info!("Inicializando BinanceApi");
        info!("API Key length: {}", api_key.len());
        info!("API Secret length: {}", api_secret.len());
        
        // Remover cualquier espacio o caracter invisible
        let api_key = api_key.trim().to_string();
        let api_secret = api_secret.trim().to_string();
        
        Self {
            client: reqwest::Client::new(),
            api_key,
            api_secret,
        }
    }

    pub async fn get_account_info(&self) -> Result<AccountInfo, Box<dyn Error>> {
        info!("Obteniendo información de cuenta de Binance...");
        
        let params = vec![("timestamp", get_timestamp())];
        
        match self.send_signed_request("/api/v3/account", &params).await {
            Ok(info) => {
                info!("Información de cuenta obtenida correctamente");
                Ok(info)
            },
            Err(e) => {
                error!("Error al obtener información de cuenta: {}", e);
                Err(e)
            }
        }
    }

    pub async fn place_order(
        &self,
        symbol: &str,
        side: &str,
        order_type: &str,
        quantity: String,
        price: Option<String>,
    ) -> Result<OrderResponse, Box<dyn Error>> {
        let timestamp = get_timestamp();
        
        // Construir parámetros en orden específico
        let mut params = vec![
            ("symbol", symbol.to_string()),
            ("side", side.to_string()),
            ("type", order_type.to_string()),
            ("quantity", quantity),
            ("timestamp", timestamp),
        ];

        // Agregar precio solo para órdenes límite
        if order_type == "LIMIT" {
            if let Some(price) = price {
                params.push(("price", price));
                params.push(("timeInForce", "GTC".to_string()));
            }
        }

        // Construir query string para firma
        let query_string = params.iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");

        // Generar firma
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query_string.as_bytes());
        let signature = hex::encode(mac.finalize().into_bytes());

        // Agregar firma a los parámetros
        params.push(("signature", signature));

        info!("Enviando orden con parámetros: {:?}", params);

        // Enviar request
        let url = "https://api.binance.com/api/v3/order";
        let response = self.client
            .post(url)
            .header("X-MBX-APIKEY", &self.api_key)
            .form(&params)
            .send()
            .await?;

        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Error de Binance: {}", error_text);
            return Err(error_text.into());
        }

        let result = response.json().await?;
        Ok(result)
    }

    async fn send_signed_request<T: for<'de> serde::Deserialize<'de>>(
        &self,
        endpoint: &str,
        params: &[(impl AsRef<str>, impl AsRef<str>)],
    ) -> Result<T, Box<dyn Error>> {
        let url = format!("https://api.binance.com{}", endpoint);
        info!("Enviando request a: {}", url);
        
        // Construir query con firma
        let signature = self.sign_request(params);
        let mut query: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();
            
        query.push(("signature".to_string(), signature));
        
        // Construir headers
        let headers = self.build_headers()?;
        
        // Usar POST para crear órdenes y GET para el resto
        let response = if endpoint == "/api/v3/order" {
            self.client
                .post(&url)
                .headers(headers)
                .form(&query)  // Usar form en lugar de query para POST
                .send()
                .await?
        } else {
            self.client
                .get(&url)
                .headers(headers)
                .query(&query)
                .send()
                .await?
        };
        
        if !response.status().is_success() {
            let error_text = response.text().await?;
            error!("Error de Binance: {}", error_text);
            return Err(error_text.into());
        }
        
        let result: T = response.json().await?;
        Ok(result)
    }

    fn sign_request(&self, params: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
        // Ordenar parámetros alfabéticamente por clave
        let mut sorted_params: Vec<_> = params.iter().collect();
        sorted_params.sort_by_key(|(k, _)| k.as_ref());
        
        // Construir query string
        let query = sorted_params
            .iter()
            .map(|(k, v)| format!("{}={}", k.as_ref(), v.as_ref()))
            .collect::<Vec<_>>()
            .join("&");
            
        info!("Query string para firma: {}", query);
        
        // Generar HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query.as_bytes());
        
        hex::encode(mac.finalize().into_bytes())
    }

    fn build_headers(&self) -> Result<HeaderMap, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-MBX-APIKEY",
            HeaderValue::from_str(&self.api_key)?,
        );
        Ok(headers)
    }

    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), Box<dyn Error>> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request::<serde_json::Value>("/api/v3/order", &params).await?;
        Ok(())
    }

    pub async fn get_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse, Box<dyn Error>> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request("/api/v3/order", &params).await
    }

    pub async fn get_all_open_orders(&self) -> Result<Vec<OrderResponse>, Box<dyn Error>> {
        info!("Obteniendo todas las órdenes abiertas");
        let params = vec![
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request("/api/v3/openOrders", &params).await
    }

    pub async fn get_open_orders(&self, symbol: &str) -> Result<Vec<OrderResponse>, Box<dyn Error>> {
        info!("Obteniendo órdenes abiertas para el símbolo: {}", symbol);
        let params = vec![
            ("symbol", symbol.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request("/api/v3/openOrders", &params).await
    }

    pub async fn get_price(&self, symbol: &str) -> Result<Decimal, ExchangeError> {
        let endpoint = "/api/v3/ticker/price";
        let params = vec![("symbol", symbol)];
        
        #[derive(Deserialize)]
        struct PriceResponse {
            price: String,
        }

        let response: PriceResponse = self.send_signed_request(endpoint, &params)
            .await
            .map_err(|e| ExchangeError::Network(e.to_string()))?;
        
        Decimal::from_str(&response.price)
            .map_err(|e| ExchangeError::ParseError(e.to_string()))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct OrderResponse {
    pub symbol: String,
    #[serde(rename = "orderId")]
    pub order_id: u64,
    #[serde(rename = "type")]
    pub order_type: String,
    pub side: String,
    pub price: String,
    #[serde(rename = "origQty")]
    pub orig_qty: String,
    pub status: String,
    #[serde(rename = "executedQty")]
    pub executed_qty: String,
    #[serde(rename = "timeInForce")]
    pub time_in_force: String,
    pub time: u64,
    #[serde(rename = "updateTime")]
    pub update_time: u64,
}

fn get_timestamp() -> String {
    chrono::Utc::now()
        .timestamp_millis()
        .to_string()
} 