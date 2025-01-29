pub mod types;
pub mod errors;
pub mod binance;
pub mod kucoin;

pub use types::*;
pub use errors::*;
pub use binance::BinanceExchange;
pub use kucoin::KuCoinExchange;

use std::sync::Arc;

pub struct ExchangeManager {
    exchanges: Vec<Arc<dyn Exchange>>,
}

impl ExchangeManager {
    pub fn new() -> Self {
        Self {
            exchanges: Vec::new(),
        }
    }
    
    pub fn add_exchange(&mut self, exchange: Arc<dyn Exchange>) {
        self.exchanges.push(exchange);
    }
    
    pub async fn execute_order(&self, order: OrderRequest) -> Result<Order, ExchangeError> {
        // Implementar lógica para ejecutar orden en el exchange apropiado
        todo!()
    }
} 