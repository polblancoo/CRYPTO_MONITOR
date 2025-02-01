use hmac::{Hmac, Mac};
use reqwest::header::{HeaderMap, HeaderValue};
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use std::error::Error;
use tracing::{info, error};
use rust_decimal::Decimal;
use std::str::FromStr;
use super::errors::ExchangeError;
use async_trait::async_trait;
use super::types::{
    Exchange, 
    Order, 
    OrderSide, 
    OrderType, 
    OrderStatus,
    Balance as ExchangeBalance
};
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::task::{Context, Poll};

type HmacSha256 = Hmac<Sha256>;

#[derive(Debug, Serialize, Deserialize)]
pub struct BinanceBalance {
    pub asset: String,
    pub free: String,
    pub locked: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountInfo {
    pub balances: Vec<BinanceBalance>,
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

    pub async fn get_account_info(&self) -> Result<AccountInfo, ExchangeError> {
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
    ) -> Result<OrderResponse, ExchangeError> {
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

    pub async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), ExchangeError> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request::<serde_json::Value>("/api/v3/order", &params)
            .method(reqwest::Method::DELETE)
            .send()
            .await?;
        Ok(())
    }

    pub async fn get_order(&self, symbol: &str, order_id: &str) -> Result<OrderResponse, ExchangeError> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request("/api/v3/order", &params).await
    }

    pub async fn get_all_open_orders(&self) -> Result<Vec<OrderResponse>, ExchangeError> {
        info!("Obteniendo todas las órdenes abiertas");
        let params = vec![
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request("/api/v3/openOrders", &params).await
    }

    pub async fn get_open_orders(&self, symbol: &str) -> Result<Vec<OrderResponse>, ExchangeError> {
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

    fn sign_request(&self, params: &[(impl AsRef<str>, impl AsRef<str>)]) -> String {
        // Crear una copia mutable de los parámetros
        let mut sorted_params: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();
        
        // Ordenar parámetros alfabéticamente por clave
        sorted_params.sort_by(|(a, _), (b, _)| a.cmp(b));
        
        // Construir query string
        let query = sorted_params
            .iter()
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join("&");
            
        info!("Query string para firma: {}", query);
        
        // Generar HMAC-SHA256
        let mut mac = HmacSha256::new_from_slice(self.api_secret.as_bytes())
            .expect("HMAC can take key of any size");
        mac.update(query.as_bytes());
        
        hex::encode(mac.finalize().into_bytes())
    }

    fn send_signed_request<T>(&self, endpoint: &str, params: &[(impl AsRef<str>, impl AsRef<str>)]) -> RequestBuilder<T>
    where
        T: serde::de::DeserializeOwned + Send + 'static,
    {
        let url = format!("https://api.binance.com{}", endpoint);
        info!("Enviando request a: {}", url);
        
        // Crear una copia mutable de los parámetros
        let mut query: Vec<(String, String)> = params
            .iter()
            .map(|(k, v)| (k.as_ref().to_string(), v.as_ref().to_string()))
            .collect();
            
        // Ordenar parámetros antes de generar la firma
        query.sort_by(|(a, _), (b, _)| a.cmp(b));
        
        // Generar firma con los parámetros ordenados
        let signature = self.sign_request(&query);
        
        // Agregar firma a los parámetros
        query.push(("signature".to_string(), signature));
        
        let headers = self.build_headers()
            .expect("Error al construir headers");

        RequestBuilder {
            client: &self.client,
            url,
            headers,
            query,
            method: reqwest::Method::GET,
            _marker: std::marker::PhantomData,
        }
    }

    fn build_headers(&self) -> Result<HeaderMap, Box<dyn Error>> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-MBX-APIKEY",
            HeaderValue::from_str(&self.api_key)?,
        );
        Ok(headers)
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

#[async_trait]
impl Exchange for BinanceApi {
    async fn get_balance(&self, asset: &str) -> Result<Vec<ExchangeBalance>, ExchangeError> {
        let params = vec![("timestamp", get_timestamp())];
        let account: AccountInfo = self.send_signed_request("/api/v3/account", &params).await?;
        
        let balances = account.balances
            .into_iter()
            .filter(|b| asset.is_empty() || b.asset == asset)
            .filter(|b| {
                let free = Decimal::from_str(&b.free).unwrap_or_default();
                let locked = Decimal::from_str(&b.locked).unwrap_or_default();
                !free.is_zero() || !locked.is_zero()
            })
            .map(|b| ExchangeBalance {
                asset: b.asset,
                free: Decimal::from_str(&b.free).unwrap_or_default(),
                locked: Decimal::from_str(&b.locked).unwrap_or_default(),
            })
            .collect();
            
        Ok(balances)
    }

    async fn place_order(
        &self,
        symbol: &str,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Order, ExchangeError> {
        let side_str = match side {
            OrderSide::Buy => "BUY",
            OrderSide::Sell => "SELL",
        };

        let type_str = match order_type {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            _ => return Err(ExchangeError::Exchange("Tipo de orden no soportado".into())),
        };

        let mut params = vec![
            ("symbol", symbol.to_string()),
            ("side", side_str.to_string()),
            ("type", type_str.to_string()),
            ("quantity", quantity.to_string()),
            ("timestamp", get_timestamp()),
        ];

        if let Some(price) = price {
            params.push(("price", price.to_string()));
            params.push(("timeInForce", "GTC".to_string()));
        }

        let response: OrderResponse = self.send_signed_request("/api/v3/order", &params).await?;
        
        Ok(Order {
            id: response.order_id.to_string(),
            symbol: response.symbol,
            order_type,
            side,
            price: Decimal::from_str(&response.price).ok(),
            quantity: Decimal::from_str(&response.orig_qty).unwrap_or_default(),
            filled_quantity: Decimal::from_str(&response.executed_qty).unwrap_or_default(),
            status: OrderStatus::New, // Convertir según el status de la respuesta
            created_at: chrono::DateTime::from_timestamp(response.time as i64 / 1000, 0)
                .unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(response.update_time as i64 / 1000, 0)
                .unwrap_or_default(),
        })
    }

    async fn get_order(&self, symbol: &str, order_id: &str) -> Result<Order, ExchangeError> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        let response: OrderResponse = self.send_signed_request("/api/v3/order", &params).await?;
        
        // Convertir la respuesta al tipo Order
        Ok(Order {
            id: response.order_id.to_string(),
            symbol: response.symbol,
            order_type: OrderType::Market, // Convertir según el tipo de la respuesta
            side: if response.side == "BUY" { OrderSide::Buy } else { OrderSide::Sell },
            price: Decimal::from_str(&response.price).ok(),
            quantity: Decimal::from_str(&response.orig_qty).unwrap_or_default(),
            filled_quantity: Decimal::from_str(&response.executed_qty).unwrap_or_default(),
            status: OrderStatus::New, // Convertir según el status de la respuesta
            created_at: chrono::DateTime::from_timestamp(response.time as i64 / 1000, 0)
                .unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(response.update_time as i64 / 1000, 0)
                .unwrap_or_default(),
        })
    }

    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<Order>, ExchangeError> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("timestamp", get_timestamp()),
        ];

        let response: Vec<OrderResponse> = self.send_signed_request("/api/v3/openOrders", &params).await?;
        
        // Convertir las respuestas al tipo Order
        Ok(response.into_iter().map(|r| Order {
            id: r.order_id.to_string(),
            symbol: r.symbol,
            order_type: OrderType::Market, // Convertir según el tipo de la respuesta
            side: if r.side == "BUY" { OrderSide::Buy } else { OrderSide::Sell },
            price: Decimal::from_str(&r.price).ok(),
            quantity: Decimal::from_str(&r.orig_qty).unwrap_or_default(),
            filled_quantity: Decimal::from_str(&r.executed_qty).unwrap_or_default(),
            status: OrderStatus::New, // Convertir según el status de la respuesta
            created_at: chrono::DateTime::from_timestamp(r.time as i64 / 1000, 0)
                .unwrap_or_default(),
            updated_at: chrono::DateTime::from_timestamp(r.update_time as i64 / 1000, 0)
                .unwrap_or_default(),
        }).collect())
    }

    async fn get_price(&self, symbol: &str) -> Result<Decimal, ExchangeError> {
        let params = vec![("symbol", symbol.to_string())];
        
        #[derive(Deserialize)]
        struct PriceResponse {
            price: String,
        }

        let response: PriceResponse = self.send_signed_request("/api/v3/ticker/price", &params).await?;
        
        Decimal::from_str(&response.price)
            .map_err(|e| ExchangeError::ParseError(e.to_string()))
    }

    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), ExchangeError> {
        let params = vec![
            ("symbol", symbol.to_string()),
            ("orderId", order_id.to_string()),
            ("timestamp", get_timestamp()),
        ];

        self.send_signed_request::<serde_json::Value>("/api/v3/order", &params)
            .method(reqwest::Method::DELETE)
            .send()
            .await?;
        Ok(())
    }
}

struct RequestBuilder<'a, T> {
    client: &'a reqwest::Client,
    url: String,
    headers: HeaderMap,
    query: Vec<(String, String)>,
    method: reqwest::Method,
    _marker: std::marker::PhantomData<T>,
}

impl<'a, T> RequestBuilder<'a, T>
where
    T: serde::de::DeserializeOwned + 'static,
{
    pub fn method(mut self, method: reqwest::Method) -> Self {
        self.method = method.clone();
        self
    }

    pub fn send(self) -> RequestFuture<'a, T> {
        RequestFuture {
            builder: self,
            future: None,
        }
    }
}

impl<'a, T> IntoFuture for RequestBuilder<'a, T>
where
    T: serde::de::DeserializeOwned + Send + 'static,
{
    type Output = Result<T, ExchangeError>;
    type IntoFuture = RequestFuture<'a, T>;

    fn into_future(self) -> Self::IntoFuture {
        self.send()
    }
}

pub struct RequestFuture<'a, T> {
    builder: RequestBuilder<'a, T>,
    future: Option<Pin<Box<dyn Future<Output = Result<T, ExchangeError>> + Send + 'a>>>,
}

impl<'a, T> Future for RequestFuture<'a, T>
where
    T: serde::de::DeserializeOwned + 'static + Send,
{
    type Output = Result<T, ExchangeError>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        
        if this.future.is_none() {
            let request = this.builder.client
                .request(this.builder.method.clone(), &this.builder.url)
                .headers(this.builder.headers.clone());

            let request = if this.builder.method == reqwest::Method::POST {
                request.form(&this.builder.query)
            } else {
                request.query(&this.builder.query)
            };

            let fut = Box::pin(async move {
                let response = request.send().await?;
                
                if !response.status().is_success() {
                    let error_text = response.text().await?;
                    error!("Error de Binance: {}", error_text);
                    return Err(ExchangeError::Api(error_text));
                }
                
                let result = response.json().await?;
                Ok(result)
            });

            this.future = Some(fut);
        }

        if let Some(fut) = &mut this.future {
            fut.as_mut().poll(cx)
        } else {
            Poll::Ready(Err(ExchangeError::Exchange("Future no inicializado".into())))
        }
    }
} 