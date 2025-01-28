use crate::{
    crypto_api::CryptoAPI,
    models::{CryptoPrice, PriceAlert},
    notify::NotificationService,
    db::Database,
};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::time;
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

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        info!("Iniciando monitor de precios...");
        let mut interval = time::interval(Duration::from_secs(self.check_interval));

        loop {
            interval.tick().await;
            if let Err(e) = self.check_all_alerts().await {
                error!("Error al verificar alertas: {}", e);
            }
        }
    }

    async fn check_all_alerts(&self) -> Result<(), Box<dyn Error>> {
        info!("Verificando alertas activas...");
        let alerts = self.db.get_active_alerts()?;
        
        // Agrupar alertas por símbolo para minimizar llamadas a la API
        let mut symbol_alerts: std::collections::HashMap<String, Vec<&PriceAlert>> = std::collections::HashMap::new();
        for alert in &alerts {
            symbol_alerts
                .entry(alert.symbol.clone())
                .or_default()
                .push(alert);
        }

        for (symbol, alerts) in symbol_alerts {
            match self.api.get_price(&symbol).await {
                Ok(price) => {
                    for alert in alerts {
                        if self.should_trigger_alert(&price, alert) {
                            info!("¡Alerta disparada! Symbol: {}, Price: {}", symbol, price.price);
                            
                            // Enviar notificación
                            if let Err(e) = self.send_alert_notification(alert, &price).await {
                                error!("Error al enviar notificación: {}", e);
                            }

                            // Marcar alerta como disparada
                            if let Err(e) = self.db.mark_alert_triggered(alert.id.unwrap()) {
                                error!("Error al marcar alerta como disparada: {}", e);
                            }
                        }
                    }
                }
                Err(e) => {
                    error!("Error al obtener precio para {}: {}", symbol, e);
                }
            }
        }

        Ok(())
    }

    fn should_trigger_alert(&self, price: &CryptoPrice, alert: &PriceAlert) -> bool {
        match alert.condition {
            crate::models::AlertCondition::Above => price.price > alert.target_price,
            crate::models::AlertCondition::Below => price.price < alert.target_price,
        }
    }

    async fn send_alert_notification(&self, alert: &PriceAlert, price: &CryptoPrice) -> Result<(), Box<dyn Error>> {
        let message = format!(
            "🚨 ¡Alerta de Precio!\n\n\
             Símbolo: {}\n\
             Precio Actual: ${:.2}\n\
             Precio Objetivo: ${:.2}\n\
             Condición: {:?}",
            alert.symbol, price.price, alert.target_price, alert.condition
        );

        self.notification_service.send_alert(alert.user_id, &message).await?;
        Ok(())
    }
} 