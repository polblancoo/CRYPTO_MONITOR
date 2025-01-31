use std::str::FromStr;
use super::{types::*, errors::*};
use rust_decimal::Decimal;
use tokio::sync::OnceCell;
use crate::exchanges::binance_api::BinanceApi;
use async_trait::async_trait;
use chrono::Utc;
use tracing::{info, error};
use std::error::Error;

pub struct BinanceExchange {
    api: BinanceApi,
    account_info: OnceCell<()>,
}

impl BinanceExchange {
    pub fn new(credentials: ExchangeCredentials) -> Result<Self, ExchangeError> {
        info!("Creando nueva instancia de BinanceExchange");
        Ok(Self {
            api: BinanceApi::new(credentials.api_key, credentials.api_secret),
            account_info: OnceCell::new(),
        })
    }

    pub async fn get_price(&self, symbol: &str) -> Result<Decimal, Box<dyn Error + Send + Sync>> {
        let price = self.api.get_price(symbol).await?;
        Ok(price)
    }
}

#[async_trait]
impl Exchange for BinanceExchange {
    async fn get_balance(&self, asset: &str) -> Result<Vec<Balance>, ExchangeError> {
        info!("Obteniendo balance para asset: {}", if asset.is_empty() { "todos" } else { asset });
        
        let account_info = self.api.get_account_info()
            .await
            .map_err(|e| {
                error!("Error al obtener información de cuenta: {}", e);
                ExchangeError::Exchange(e.to_string())
            })?;
            
        info!("Balances recibidos: {:?}", account_info.balances);
        
        if asset.is_empty() {
            // Retornar todos los balances no nulos
            let balances: Vec<Balance> = account_info.balances
                .iter()
                .filter(|b| {
                    let free = Decimal::from_str(&b.free).unwrap_or_default();
                    let locked = Decimal::from_str(&b.locked).unwrap_or_default();
                    let has_balance = !free.is_zero() || !locked.is_zero();
                    info!("Asset {}: free={}, locked={}, incluido={}", 
                        b.asset, free, locked, has_balance);
                    has_balance
                })
                .map(|b| Balance {
                    asset: b.asset.clone(),
                    free: Decimal::from_str(&b.free).unwrap_or_default(),
                    locked: Decimal::from_str(&b.locked).unwrap_or_default(),
                })
                .collect();

            info!("Balances filtrados: {:?}", balances);
            
            if balances.is_empty() {
                error!("No se encontraron balances con saldo");
                return Err(ExchangeError::AssetNotFound("No balances found".into()));
            }

            Ok(balances)
        } else {
            // Buscar un balance específico
            let balance = account_info.balances
                .iter()
                .find(|b| b.asset == asset)
                .ok_or(ExchangeError::AssetNotFound(asset.to_string()))?;
                
            Ok(vec![Balance {
                asset: balance.asset.clone(),
                free: Decimal::from_str(&balance.free)?,
                locked: Decimal::from_str(&balance.locked)?,
            }])
        }
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

        let order_type_str = match order_type {
            OrderType::Market => "MARKET",
            OrderType::Limit => "LIMIT",
            OrderType::StopLoss => "STOP_LOSS",
            OrderType::StopLossLimit => "STOP_LOSS_LIMIT",
            OrderType::TakeProfit => "TAKE_PROFIT",
            OrderType::TakeProfitLimit => "TAKE_PROFIT_LIMIT",
        };

        let order = self.api.place_order(
            symbol,
            side_str,
            order_type_str,
            quantity.to_string(),
            price.map(|p| p.to_string()),
        ).await.map_err(|e| ExchangeError::Exchange(e.to_string()))?;

        Ok(Order {
            id: order.order_id.to_string(),
            symbol: order.symbol,
            side,
            order_type,
            price,
            quantity,
            filled_quantity: Decimal::from_str("0")?,
            status: OrderStatus::New,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    
    async fn cancel_order(&self, symbol: &str, order_id: &str) -> Result<(), ExchangeError> {
        self.api.cancel_order(symbol, order_id)
            .await
            .map_err(|e| ExchangeError::Exchange(e.to_string()))
    }
    
    async fn get_order(&self, symbol: &str, order_id: &str) -> Result<Order, ExchangeError> {
        let order = self.api.get_order(symbol, order_id)
            .await
            .map_err(|e| ExchangeError::Exchange(e.to_string()))?;

        Ok(Order {
            id: order.order_id.to_string(),
            symbol: order.symbol,
            side: match order.side.as_str() {
                "BUY" => OrderSide::Buy,
                "SELL" => OrderSide::Sell,
                _ => return Err(ExchangeError::Exchange("Invalid order side".into())),
            },
            order_type: match order.order_type.as_str() {
                "MARKET" => OrderType::Market,
                "LIMIT" => OrderType::Limit,
                "STOP_LOSS" => OrderType::StopLoss,
                "STOP_LOSS_LIMIT" => OrderType::StopLossLimit,
                "TAKE_PROFIT" => OrderType::TakeProfit,
                "TAKE_PROFIT_LIMIT" => OrderType::TakeProfitLimit,
                _ => return Err(ExchangeError::Exchange("Invalid order type".into())),
            },
            price: Some(Decimal::from_str(&order.price)?),
            quantity: Decimal::from_str(&order.orig_qty)?,
            filled_quantity: Decimal::from_str("0")?,
            status: match order.status.as_str() {
                "NEW" => OrderStatus::New,
                "FILLED" => OrderStatus::Filled,
                "CANCELED" => OrderStatus::Canceled,
                "PARTIALLY_FILLED" => OrderStatus::New,
                _ => OrderStatus::Canceled,
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        })
    }
    
    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<Order>, ExchangeError> {
        info!("Obteniendo órdenes abiertas...");
        
        let orders = if symbol.is_empty() {
            self.api.get_all_open_orders().await
        } else {
            self.api.get_open_orders(symbol).await
        }.map_err(|e| ExchangeError::Exchange(e.to_string()))?;

        orders.into_iter()
            .map(|order| {
                Ok(Order {
                    id: order.order_id.to_string(),
                    symbol: order.symbol,
                    side: match order.side.as_str() {
                        "BUY" => OrderSide::Buy,
                        "SELL" => OrderSide::Sell,
                        _ => return Err(ExchangeError::Exchange("Invalid order side".into())),
                    },
                    order_type: match order.order_type.as_str() {
                        "MARKET" => OrderType::Market,
                        "LIMIT" => OrderType::Limit,
                        "STOP_LOSS" => OrderType::StopLoss,
                        "STOP_LOSS_LIMIT" => OrderType::StopLossLimit,
                        "TAKE_PROFIT" => OrderType::TakeProfit,
                        "TAKE_PROFIT_LIMIT" => OrderType::TakeProfitLimit,
                        _ => return Err(ExchangeError::Exchange("Invalid order type".into())),
                    },
                    price: Some(Decimal::from_str(&order.price)?),
                    quantity: Decimal::from_str(&order.orig_qty)?,
                    filled_quantity: Decimal::from_str(&order.executed_qty)?,
                    status: match order.status.as_str() {
                        "NEW" => OrderStatus::New,
                        "PARTIALLY_FILLED" => OrderStatus::PartiallyFilled,
                        "FILLED" => OrderStatus::Filled,
                        "CANCELED" => OrderStatus::Canceled,
                        _ => OrderStatus::Canceled,
                    },
                    created_at: Utc::now(),
                    updated_at: Utc::now(),
                })
            })
            .collect()
    }

    async fn get_price(&self, symbol: &str) -> Result<Decimal, ExchangeError> {
        self.api.get_price(symbol)
            .await
            .map_err(|e| ExchangeError::Exchange(e.to_string()))
    }
} 