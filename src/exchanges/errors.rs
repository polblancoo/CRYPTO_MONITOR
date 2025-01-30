use thiserror::Error;

#[derive(Error, Debug)]
pub enum ExchangeError {
    #[error("Error de autenticación: {0}")]
    Authentication(String),

    #[error("Error de red: {0}")]
    Network(String),

    #[error("Asset no encontrado: {0}")]
    AssetNotFound(String),

    #[error("Error de formato: {0}")]
    Format(String),

    #[error("Error del exchange: {0}")]
    Exchange(String),

    #[error("Fondos insuficientes")]
    InsufficientFunds,

    #[error("Credenciales faltantes: {0}")]
    MissingCredentials(String),
}

impl From<rust_decimal::Error> for ExchangeError {
    fn from(err: rust_decimal::Error) -> Self {
        ExchangeError::Format(err.to_string())
    }
}

impl From<chrono::ParseError> for ExchangeError {
    fn from(err: chrono::ParseError) -> Self {
        ExchangeError::Format(err.to_string())
    }
}

impl From<binance::errors::Error> for ExchangeError {
    fn from(err: binance::errors::Error) -> Self {
        ExchangeError::Exchange(err.to_string())
    }
} 