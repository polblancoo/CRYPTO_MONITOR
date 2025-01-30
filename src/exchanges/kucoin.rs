use super::{types::*, errors::*};
use rust_decimal::Decimal;
use kucoin_rs::{kucoin::*, client::*};
use std::str::FromStr;

pub struct KuCoinExchange {
    client: KucoinClient,
}

impl KuCoinExchange {
    pub fn new(credentials: ExchangeCredentials) -> Result<Self, ExchangeError> {
        let passphrase = credentials.passphrase
            .ok_or(ExchangeError::MissingCredentials("KuCoin requires passphrase".into()))?;
            
        let client = KucoinClient::new(
            credentials.api_key,
            credentials.api_secret,
            passphrase,
        );
        
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl Exchange for KuCoinExchange {
    async fn get_balance(&self, asset: &str) -> Result<Balance, ExchangeError> {
        let accounts = self.client.get_accounts(Some(asset), None).await?;
        let account = accounts.first()
            .ok_or(ExchangeError::AssetNotFound(asset.to_string()))?;
        
        Ok(Balance {
            asset: account.currency.clone(),
            free: Decimal::from_str(&account.available)?,
            locked: Decimal::from_str(&account.holds)?,
        })
    }
    
    async fn place_order(
        &self,
        symbol: &str,
        side: OrderSide,
        order_type: OrderType,
        quantity: Decimal,
        price: Option<Decimal>,
    ) -> Result<Order, ExchangeError> {
        let kucoin_side = match side {
            OrderSide::Buy => Side::Buy,
            OrderSide::Sell => Side::Sell,
        };

        let kucoin_type = match order_type {
            OrderType::Market => OrderType::Market,
            OrderType::Limit => OrderType::Limit,
            _ => return Err(ExchangeError::Exchange("Tipo de orden no soportada".into())),
        };

        let order_request = CreateOrderRequest {
            client_oid: None,
            side: kucoin_side,
            symbol: symbol.to_string(),
            order_type: kucoin_type,
            price: price.map(|p| p.to_string()),
            size: Some(quantity.to_string()),
            remark: None,
            stop: None,
            stop_price: None,
            time_in_force: None,
            cancel_after: None,
            post_only: None,
            hidden: None,
            iceberg: None,
            visible_size: None,
        };

        let response = self.client.create_order(order_request).await?;

        Ok(Order {
            id: response.order_id,
            symbol: symbol.to_string(),
            order_type,
            side,
            price,
            quantity,
            filled_quantity: Decimal::from_str("0.0")?, // KuCoin no devuelve esto en la creación
            status: OrderStatus::New,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        })
    }
    
    async fn cancel_order(&self, _symbol: &str, order_id: &str) -> Result<(), ExchangeError> {
        self.client.cancel_order(order_id).await?;
        Ok(())
    }
    
    async fn get_order(&self, _symbol: &str, order_id: &str) -> Result<Order, ExchangeError> {
        let order = self.client.get_order(order_id).await?;
        
        Ok(Order {
            id: order.id,
            symbol: order.symbol,
            order_type: match order.order_type.as_str() {
                "market" => OrderType::Market,
                "limit" => OrderType::Limit,
                _ => OrderType::Market,
            },
            side: match order.side.as_str() {
                "buy" => OrderSide::Buy,
                "sell" => OrderSide::Sell,
                _ => return Err(ExchangeError::Exchange("Side desconocido".into())),
            },
            price: order.price.and_then(|p| Decimal::from_str(&p).ok()),
            quantity: Decimal::from_str(&order.size)?,
            filled_quantity: Decimal::from_str(&order.deal_size)?,
            status: match order.is_active {
                true => OrderStatus::New,
                false => if order.deal_size == order.size {
                    OrderStatus::Filled
                } else {
                    OrderStatus::Canceled
                },
            },
            created_at: chrono::DateTime::parse_from_rfc3339(&order.created_at)?.into(),
            updated_at: chrono::Utc::now(),
        })
    }
    
    async fn get_open_orders(&self, symbol: &str) -> Result<Vec<Order>, ExchangeError> {
        let orders = self.client.get_orders(Some(symbol), None).await?;
        
        let mut result = Vec::new();
        for order in orders {
            result.push(Order {
                id: order.id,
                symbol: order.symbol,
                order_type: match order.order_type.as_str() {
                    "market" => OrderType::Market,
                    "limit" => OrderType::Limit,
                    _ => OrderType::Market,
                },
                side: match order.side.as_str() {
                    "buy" => OrderSide::Buy,
                    "sell" => OrderSide::Sell,
                    _ => continue,
                },
                price: order.price.and_then(|p| Decimal::from_str(&p).ok()),
                quantity: Decimal::from_str(&order.size)?,
                filled_quantity: Decimal::from_str(&order.deal_size)?,
                status: OrderStatus::New,
                created_at: chrono::DateTime::parse_from_rfc3339(&order.created_at)?.into(),
                updated_at: chrono::Utc::now(),
            });
        }
        
        Ok(result)
    }
} 