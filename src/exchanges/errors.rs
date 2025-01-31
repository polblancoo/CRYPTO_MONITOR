use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ExchangeError {
    Exchange(String),
    ParseError(String),
    Network(String),
    Authentication(String),
    RateLimit(String),
    Format(String),
    AssetNotFound(String),
    InsufficientFunds,
    MissingCredentials(String),
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Exchange(msg) => write!(f, "Error del exchange: {}", msg),
            Self::ParseError(msg) => write!(f, "Error de parseo: {}", msg),
            Self::Network(msg) => write!(f, "Error de red: {}", msg),
            Self::Authentication(msg) => write!(f, "Error de autenticación: {}", msg),
            Self::RateLimit(msg) => write!(f, "Error de límite de tasa: {}", msg),
            Self::Format(msg) => write!(f, "Error de formato: {}", msg),
            Self::AssetNotFound(msg) => write!(f, "Asset no encontrado: {}", msg),
            Self::InsufficientFunds => write!(f, "Fondos insuficientes"),
            Self::MissingCredentials(msg) => write!(f, "Credenciales faltantes: {}", msg),
        }
    }
}

impl Error for ExchangeError {}

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