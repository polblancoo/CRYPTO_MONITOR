use super::{types::ExchangeCredentials, errors::ExchangeError};
use std::env;
use tracing::{info, error};

pub fn load_binance_credentials() -> Result<ExchangeCredentials, ExchangeError> {
    info!("Cargando credenciales de Binance...");
    
    let api_key = match env::var("BINANCE_API_KEY") {
        Ok(key) => {
            info!("API Key encontrada, longitud: {}", key.len());
            if key.len() != 64 {
                return Err(ExchangeError::Exchange(
                    format!("API Key debe tener 64 caracteres, tiene {}", key.len())
                ));
            }
            key.trim().to_string()
        },
        Err(e) => {
            error!("Error al cargar BINANCE_API_KEY: {}", e);
            return Err(ExchangeError::Exchange("BINANCE_API_KEY no encontrada".into()));
        }
    };
        
    let api_secret = match env::var("BINANCE_API_SECRET") {
        Ok(secret) => {
            info!("API Secret encontrada, longitud: {}", secret.len());
            if secret.len() != 64 {
                return Err(ExchangeError::Exchange(
                    format!("API Secret debe tener 64 caracteres, tiene {}", secret.len())
                ));
            }
            secret.trim().to_string()
        },
        Err(e) => {
            error!("Error al cargar BINANCE_API_SECRET: {}", e);
            return Err(ExchangeError::Exchange("BINANCE_API_SECRET no encontrada".into()));
        }
    };
    
    info!("Credenciales de Binance cargadas correctamente");
    
    Ok(ExchangeCredentials {
        api_key,
        api_secret,
    })
} 