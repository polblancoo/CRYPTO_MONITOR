use super::{types::*, errors::*};
use rust_decimal::Decimal;
use binance::{api::*, config::*, market::*, account::*};

pub struct BinanceExchange {
    account: Account,
    market: Market,
}

impl BinanceExchange {
    pub fn new(credentials: ExchangeCredentials) -> Result<Self, ExchangeError> {
        let config = Config::default()
            .api_key(Some(credentials.api_key))
            .secret_key(Some(credentials.api_secret));
            
        let account = Account::new(Some(config.clone()));
        let market = Market::new(Some(config));
        
        Ok(Self { account, market })
    }
}

#[async_trait::async_trait]
impl Exchange for BinanceExchange {
    async fn get_balance(&self, asset: &str) -> Result<Balance, ExchangeError> {
        let account = self.account.get_account()?;
        let balance = account.balances
            .iter()
            .find(|b| b.asset == asset)
            .ok_or(ExchangeError::AssetNotFound(asset.to_string()))?;
            
        Ok(Balance {
            asset: balance.asset.clone(),
            free: Decimal::from_str(&balance.free)?,
            locked: Decimal::from_str(&balance.locked)?,
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
        // Implementar lógica de orden
        todo!()
    }
    
    // Implementar resto de métodos...
} 