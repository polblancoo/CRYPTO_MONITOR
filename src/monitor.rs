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
        let alerts = self.db.get_active_alerts()?;
        info!("Encontradas {} alertas activas", alerts.len());
        
        // Agrupar alertas por símbolo para minimizar llamadas a la API
        let mut symbol_alerts: std::collections::HashMap<String, Vec<&PriceAlert>> = std::collections::HashMap::new();
        for alert in &alerts {
            info!("Procesando alerta: ID={}, Symbol={}, Target=${}, Condition={:?}", 
                alert.id.unwrap_or(-1), alert.symbol, alert.target_price, alert.condition);
            symbol_alerts
                .entry(alert.symbol.clone())
                .or_default()
                .push(alert);
        }

        for (symbol, alerts) in symbol_alerts {
            info!("Obteniendo precio para {}", symbol);
            match self.api.get_price(&symbol).await {
                Ok(price) => {
                    info!("Precio actual de {}: ${}", symbol, price.price);
                    for alert in alerts {
                        info!("Evaluando alerta: Target=${} vs Actual=${}", alert.target_price, price.price);
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
                        } else {
                            info!("Condición no cumplida para alerta ID={}", alert.id.unwrap_or(-1));
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

    async fn send_alert_notification(&self, alert: &PriceAlert, price: &CryptoPrice) -> Result<(), Box<dyn Error + Send + Sync>> {
        // Obtener el telegram_chat_id del usuario
        let user = self.db.get_user_telegram_chat_id(alert.user_id)?
            .ok_or_else(|| format!("No se encontró telegram_chat_id para el usuario {}", alert.user_id))?;

        info!("Preparando notificación para usuario_id: {} (telegram_chat_id: {})", alert.user_id, user);
        
        let message = format!(
            "🚨 ¡Alerta de Precio!\n\n\
             Símbolo: {}\n\
             Precio Actual: ${:.2}\n\
             Precio Objetivo: ${:.2}\n\
             Condición: {:?}",
            alert.symbol, price.price, alert.target_price, alert.condition
        );

        match self.notification_service.send_alert(user, &message).await {
            Ok(_) => {
                info!("Notificación enviada exitosamente al usuario {} (telegram: {})", alert.user_id, user);
                Ok(())
            }
            Err(e) => {
                error!("Error al enviar notificación al usuario {} (telegram: {}): {}", alert.user_id, user, e);
                Err(e)
            }
        }
    }
} 