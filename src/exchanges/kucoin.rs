use super::{types::*, errors::*};
use rust_decimal::Decimal;
use kucoin_rs::{client::*, trade::*};

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
        )?;
        
        Ok(Self { client })
    }
}

#[async_trait::async_trait]
impl Exchange for KuCoinExchange {
    async fn get_balance(&self, asset: &str) -> Result<Balance, ExchangeError> {
        let account = self.client.get_account(asset)?;
        
        Ok(Balance {
            asset: account.currency,
            free: Decimal::from_str(&account.available)?,
            locked: Decimal::from_str(&account.holds)?,
        })
    }
    
    // Implementar resto de métodos...
} 