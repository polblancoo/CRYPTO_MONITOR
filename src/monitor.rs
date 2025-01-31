use crate::{
    crypto_api::CryptoAPI,
    models::{CryptoPrice, PriceAlert, AlertType, AlertCondition},
    notify::NotificationService,
    db::Database,
};
use std::{error::Error, sync::Arc};
use tokio::time::{self, Duration};
use tracing::{info, error};

pub struct PriceMonitor {
    api: CryptoAPI,
    notification_service: NotificationService,
    db: Arc<Database>,
    check_interval: u64,
}

impl PriceMonitor {
    pub fn new(api: CryptoAPI, notification_service: NotificationService, db: Arc<Database>, check_interval: u64) -> Self {
        Self {
            api,
            notification_service,
            db,
            check_interval,
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Iniciando monitor de precios...");
        
        // Verificar el bot al inicio
        if let Err(e) = self.notification_service.verify_bot().await {
            error!("Error al verificar el bot de Telegram: {}", e);
            return Err(e);
        }

        let mut interval = time::interval(Duration::from_secs(self.check_interval));

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_alerts().await {
                error!("Error al verificar alertas: {}", e);
            }
        }
    }

    async fn check_all_alerts(&self) -> Result<(), Box<dyn Error + Send + Sync>> {
        info!("Verificando alertas activas...");
        let alerts = self.db.get_active_alerts().await?;
        info!("Encontradas {} alertas activas", alerts.len());
        
        for alert in alerts {
            match &alert.alert_type {
                AlertType::Price { target_price, condition } => {
                    info!("Evaluando alerta de precio: ID={}, Symbol={}, Target=${}, Condition={:?}", 
                        alert.id.unwrap_or(-1), alert.symbol, target_price, condition);
                    
                    if let Ok(price) = self.api.get_price(&alert.symbol).await {
                        if self.should_trigger_alert(&price, &alert) {
                            if let Err(e) = self.send_alert_notification(&alert, &price).await {
                                error!("Error al enviar notificación: {}", e);
                            }
                            if let Err(e) = self.db.mark_alert_triggered(alert.id.unwrap()).await {
                                error!("Error al marcar alerta como disparada: {}", e);
                            }
                        }
                    }
                },
                AlertType::Depeg { target_price, exchanges, .. } => {
                    info!(
                        "Evaluando alerta de depeg: ID={}, Symbol={}, Target=${}", 
                        alert.id.unwrap_or(-1), 
                        alert.symbol, 
                        target_price
                    );
                    
                    let mut prices = Vec::new();
                    for exchange in exchanges {
                        if let Ok(price) = self.api.get_price_from_exchange(&alert.symbol, exchange).await {
                            prices.push(price.price);
                        }
                    }
                    
                    if !prices.is_empty() {
                        let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
                        let min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
                        
                        let trigger_price = if (max_price - target_price).abs() > (min_price - target_price).abs() {
                            max_price
                        } else {
                            min_price
                        };
                        
                        let crypto_price = CryptoPrice {
                            symbol: alert.symbol.clone(),
                            price: trigger_price,
                            exchange: "multiple".to_string(),
                            timestamp: chrono::Utc::now().timestamp(),
                        };
                        
                        if let Err(e) = self.send_alert_notification(&alert, &crypto_price).await {
                            error!("Error al enviar notificación: {}", e);
                        }
                        if let Err(e) = self.db.mark_alert_triggered(alert.id.unwrap()).await {
                            error!("Error al marcar alerta como disparada: {}", e);
                        }
                    }
                },
                AlertType::PairDepeg { token1, token2, expected_ratio, differential } => {
                    // Implementación del manejo de alertas de par de tokens
                    if let Ok(price1) = self.api.get_price(&token1).await {
                        if let Ok(price2) = self.api.get_price(&token2).await {
                            let current_ratio = price1.price / price2.price;
                            let deviation = ((current_ratio - expected_ratio) / expected_ratio).abs() * 100.0;
                            
                            if deviation > *differential {
                                let crypto_price = CryptoPrice {
                                    symbol: format!("{}/{}", token1, token2),
                                    price: current_ratio,
                                    exchange: "ratio".to_string(),
                                    timestamp: chrono::Utc::now().timestamp(),
                                };
                                
                                if let Err(e) = self.send_alert_notification(&alert, &crypto_price).await {
                                    error!("Error al enviar notificación: {}", e);
                                }
                                if let Err(e) = self.db.mark_alert_triggered(alert.id.unwrap()).await {
                                    error!("Error al marcar alerta como disparada: {}", e);
                                }
                            }
                        }
                    }
                }
            }
        }

        Ok(())
    }

    fn should_trigger_alert(&self, price: &CryptoPrice, alert: &PriceAlert) -> bool {
        match &alert.alert_type {
            AlertType::Price { target_price, condition } => match condition {
                AlertCondition::Above => price.price > *target_price,
                AlertCondition::Below => price.price < *target_price,
            },
            AlertType::Depeg { target_price, .. } => {
                let deviation = ((price.price - target_price) / target_price).abs() * 100.0;
                deviation > 0.0
            },
            AlertType::PairDepeg { .. } => false, // Se maneja en check_pair_depeg
        }
    }

    async fn send_alert_notification(&self, alert: &PriceAlert, price: &CryptoPrice) -> Result<(), Box<dyn Error + Send + Sync>> {
        let user = self.db.get_user_by_telegram_id(alert.user_id)
            .await?
            .ok_or_else(|| format!("No se encontró usuario para el id {}", alert.user_id))?;

        let message = match &alert.alert_type {
            AlertType::Price { target_price, condition } => {
                format!(
                    "🚨 ¡Alerta de Precio!\n\n\
                     Símbolo: {}\n\
                     Precio Actual: ${:.2}\n\
                     Precio Objetivo: ${:.2}\n\
                     Condición: {:?}",
                    alert.symbol, price.price, target_price, condition
                )
            },
            AlertType::Depeg { target_price, exchanges, .. } => {
                format!(
                    "🚨 ¡Alerta de Depeg!\n\n\
                     Símbolo: {}\n\
                     Precio Actual: ${:.2}\n\
                     Precio Objetivo: ${:.2}\n\
                     Exchanges: {}",
                    alert.symbol, price.price, target_price,
                    exchanges.join(", ")
                )
            },
            AlertType::PairDepeg { token1, token2, expected_ratio, differential } => {
                format!(
                    "🚨 ¡Alerta de Depeg de Par!\n\n\
                     Par: {}/{}\n\
                     Ratio Actual: {:.4}\n\
                     Ratio Esperado: {:.4}\n\
                     Desviación: {:.2}%",
                    token1, token2,
                    price.price, // Aquí necesitarías el ratio actual real
                    expected_ratio,
                    differential
                )
            }
        };

        self.notification_service.send_alert(user.telegram_chat_id.unwrap_or(0), &message).await
    }

    async fn check_depeg_alert(&self, alert: &PriceAlert) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if let AlertType::Depeg { target_price, exchanges, .. } = &alert.alert_type {
            let mut prices = Vec::new();
            
            for exchange in exchanges {
                if let Ok(price) = self.api.get_price_from_exchange(&alert.symbol, exchange).await {
                    prices.push(price.price);
                }
            }

            if prices.is_empty() {
                return Ok(false);
            }

            let max_price = prices.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b));
            let _min_price = prices.iter().fold(f64::INFINITY, |a, &b| a.min(b));
            
            let deviation = ((max_price - target_price).abs() / target_price) * 100.0;
            
            Ok(deviation > 0.0)
        } else {
            Ok(false)
        }
    }

    async fn check_pair_depeg(&self, alert: &PriceAlert) -> Result<bool, Box<dyn Error + Send + Sync>> {
        if let AlertType::PairDepeg { token1, token2, expected_ratio, differential } = &alert.alert_type {
            let price1 = self.api.get_price(&token1).await?;
            let price2 = self.api.get_price(&token2).await?;
            
            let current_ratio = price1.price / price2.price;
            let deviation = ((current_ratio - expected_ratio) / expected_ratio).abs() * 100.0;
            
            Ok(deviation > *differential)
        } else {
            Ok(false)
        }
    }
} 