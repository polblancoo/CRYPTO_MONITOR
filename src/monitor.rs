use crate::{
    crypto_api::CryptoAPI,
    models::{CryptoPrice, PriceAlert, AlertCondition},
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
        
        // Verificar bot con reintentos
        match self.notification_service.verify_bot().await {
            Ok(_) => info!("Bot de Telegram verificado correctamente"),
            Err(e) => {
                error!("Error al verificar el bot de Telegram: {}", e);
                // Continuar de todos modos, podría recuperarse más tarde
            }
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
            info!("Evaluando alerta de precio: ID={}, Symbol={}, Target=${}, Condition={:?}", 
                alert.id.unwrap_or(-1), alert.symbol, alert.target_price, alert.condition);
            
            if let Ok(price) = self.api.get_price(&alert.symbol).await {
                if self.check_price_alert(&alert, &price).await {
                    if let Err(e) = self.send_alert_notification(&alert, &price).await {
                        error!("Error al enviar notificación: {}", e);
                    }
                    if let Err(e) = self.db.mark_alert_triggered(alert.id.unwrap()).await {
                        error!("Error al marcar alerta como disparada: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    async fn check_price_alert(&self, alert: &PriceAlert, price: &CryptoPrice) -> bool {
        match alert.condition {
            AlertCondition::Above => price.price > alert.target_price,
            AlertCondition::Below => price.price < alert.target_price,
        }
    }

    async fn send_alert_notification(&self, alert: &PriceAlert, price: &CryptoPrice) -> Result<(), Box<dyn Error + Send + Sync>> {
        let user = self.db.get_user_by_telegram_id(alert.user_id)
            .await?
            .ok_or_else(|| format!("No se encontró usuario para el id {}", alert.user_id))?;

        let message = self.format_alert_message(alert, price).await;

        self.notification_service.send_alert(user.telegram_chat_id.unwrap_or(0), &message).await
    }

    async fn format_alert_message(&self, alert: &PriceAlert, price: &CryptoPrice) -> String {
        format!(
            "🚨 *Alerta de Precio*\n\
             Symbol: {}\n\
             Condición: {} {}\n\
             Precio actual: ${:.2}",
            alert.symbol,
            match alert.condition {
                AlertCondition::Above => "mayor que",
                AlertCondition::Below => "menor que",
            },
            alert.target_price,
            price.price
        )
    }
} 