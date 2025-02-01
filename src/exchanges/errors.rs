use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum ExchangeError {
    Network(String),
    Api(String),
    Exchange(String),
    ParseError(String),
    Authentication(String),
    RateLimit(String),
    Format(String),
    AssetNotFound(String),
    InsufficientFunds,
    MissingCredentials(String),
    NotFound(String),
}

impl fmt::Display for ExchangeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Network(e) => write!(f, "Error de red: {}", e),
            Self::Api(e) => write!(f, "Error de API: {}", e),
            Self::Exchange(e) => write!(f, "Error del exchange: {}", e),
            Self::ParseError(e) => write!(f, "Error de parseo: {}", e),
            Self::Authentication(msg) => write!(f, "Error de autenticación: {}", msg),
            Self::RateLimit(msg) => write!(f, "Error de límite de tasa: {}", msg),
            Self::Format(msg) => write!(f, "Error de formato: {}", msg),
            Self::AssetNotFound(msg) => write!(f, "Asset no encontrado: {}", msg),
            Self::InsufficientFunds => write!(f, "Fondos insuficientes"),
            Self::MissingCredentials(msg) => write!(f, "Credenciales faltantes: {}", msg),
            Self::NotFound(msg) => write!(f, "No encontrado: {}", msg),
        }
    }
}

impl Error for ExchangeError {}

impl From<reqwest::Error> for ExchangeError {
    fn from(err: reqwest::Error) -> Self {
        ExchangeError::Network(err.to_string())
    }
}

impl From<Box<dyn Error>> for ExchangeError {
    fn from(err: Box<dyn Error>) -> Self {
        ExchangeError::Exchange(err.to_string())
    }
}

impl From<String> for ExchangeError {
    fn from(err: String) -> Self {
        ExchangeError::Api(err)
    }
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