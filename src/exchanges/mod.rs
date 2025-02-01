pub mod types;
pub mod errors;
pub mod binance;
pub mod config;
pub mod binance_api;
pub mod symbols;

pub use types::{Balance, OrderSide, OrderType, OrderStatus, Order, ExchangeCredentials};
pub use errors::ExchangeError;
pub use binance::BinanceExchange;
pub use symbols::*;

use std::collections::HashMap;
use std::str::FromStr;
use std::fmt;
use rust_decimal::Decimal;
use crate::exchanges::types::Exchange;
use async_trait::async_trait;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ExchangeType {
    Binance,
}

impl fmt::Display for ExchangeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ExchangeType::Binance => write!(f, "binance"),
        }
    }
}

impl FromStr for ExchangeType {
    type Err = ExchangeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "binance" => Ok(ExchangeType::Binance),
            _ => Err(ExchangeError::Exchange("Exchange no soportado".into())),
        }
    }
}

pub struct ExchangeManager {
    exchanges: HashMap<ExchangeType, Box<dyn Exchange + Send + Sync>>,
}

impl ExchangeManager {
    pub fn new() -> Result<Self, ExchangeError> {
        let mut exchanges = HashMap::new();
        
        if let Ok(credentials) = config::load_binance_credentials() {
            let binance = BinanceExchange::new(credentials)?;
            exchanges.insert(ExchangeType::Binance, Box::new(binance) as Box<dyn Exchange + Send + Sync>);
        }
        
        Ok(Self { exchanges })
    }

    pub fn get_exchange(&self, exchange_type: ExchangeType) -> Option<&(dyn Exchange + Send + Sync)> {
        self.exchanges.get(&exchange_type).map(|b| b.as_ref())
    }
    
    pub async fn get_balances(&self) -> Result<Vec<Balance>, ExchangeError> {
        let mut all_balances = Vec::new();
        
        for (_, exchange) in self.exchanges.iter() {
            if let Ok(balances) = exchange.get_balance("").await {
                all_balances.extend(balances);
            }
        }
        
        if all_balances.is_empty() {
            return Err(ExchangeError::Exchange("No balances found".into()));
        }
        
        Ok(all_balances)
    }
    
    pub async fn cancel_order(
        &self,
        exchange_type: ExchangeType,
        symbol: &str,
        order_id: &str
    ) -> Result<(), ExchangeError> {
        let exchange = self.get_exchange(exchange_type)
            .ok_or_else(|| ExchangeError::Exchange("Exchange no encontrado".into()))?;
        exchange.cancel_order(symbol, order_id).await
    }

    pub async fn get_open_orders(&self, exchange_type: ExchangeType) -> Result<Vec<Order>, ExchangeError> {
        let exchange = self.exchanges.get(&exchange_type)
            .ok_or(ExchangeError::Exchange("Exchange no encontrado".into()))?;
            
        exchange.get_open_orders("").await
    }

    pub async fn get_price(&self, symbol: &str) -> Result<Decimal, ExchangeError> {
        let binance = self.get_exchange(ExchangeType::Binance)
            .ok_or_else(|| ExchangeError::Exchange("Exchange no encontrado".into()))?;
            
        binance.get_price(symbol).await
    }
}

#[async_trait]
impl Exchange for ExchangeManager {
    async fn get_balance(&self, _asset: &str) -> Result<Vec<Balance>, ExchangeError> {
        self.get_balances().await
    }

    async fn place_order(
        &self,
        symbol: &str,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Order, ExchangeError> {
        if let Some(exchange) = self.exchanges.get(&ExchangeType::Binance) {
            exchange.place_order(symbol, side, order_type, quantity, price).await
        } else {
            Err(ExchangeError::Exchange("Exchange no encontrado".into()))
        }
    }

    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), ExchangeError> {
        if let Some(exchange) = self.exchanges.get(&ExchangeType::Binance) {
            exchange.cancel_order(symbol, order_id).await
        } else {
            Err(ExchangeError::Exchange("Exchange no encontrado".into()))
        }
    }

    async fn get_order(&self, symbol: &str, order_id: &str) -> Result<Order, ExchangeError> {
        if let Some(exchange) = self.exchanges.get(&ExchangeType::Binance) {
            exchange.get_order(symbol, order_id).await
        } else {
            Err(ExchangeError::Exchange("Exchange no encontrado".into()))
        }
    }

    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<Order>, ExchangeError> {
        if let Some(exchange) = self.exchanges.get(&ExchangeType::Binance) {
            exchange.get_open_orders(symbol).await
        } else {
            Err(ExchangeError::Exchange("Exchange no encontrado".into()))
        }
    }

    async fn get_price(&self, symbol: &str) -> Result<Decimal, ExchangeError> {
        if let Some(exchange) = self.exchanges.get(&ExchangeType::Binance) {
            exchange.get_price(symbol).await
        } else {
            Err(ExchangeError::Exchange("Exchange no encontrado".into()))
        }
    }
}

#[derive(Debug)]
pub struct OrderRequest {
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: rust_decimal::Decimal,
    pub price: Option<rust_decimal::Decimal>,
} 