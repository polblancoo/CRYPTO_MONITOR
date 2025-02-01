use serde::{Deserialize, Serialize};
use rusqlite::{types::{FromSql, FromSqlResult, ValueRef, ToSql, ToSqlOutput}, Result as SqliteResult};
use rust_decimal::Decimal;
use chrono::{DateTime, Utc};
use std::fmt;
use std::str::FromStr;

#[derive(Debug, Serialize, Deserialize)]
pub struct User {
    pub id: i64,
    pub username: String,
    #[serde(skip_serializing)]
    pub password_hash: String,
    pub api_key: Option<String>,
    pub telegram_chat_id: Option<i64>,
    pub created_at: i64,
    pub last_login: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum AlertType {
    Price {
        target_price: f64,
        condition: AlertCondition,
    },
    Depeg {
        target_price: f64,
        differential: f64,  // porcentaje permitido de desviación
        exchanges: Vec<String>,  // lista de exchanges a monitorear
    },
    PairDepeg {
        token1: String,
        token2: String,
        expected_ratio: f64,
        differential: f64,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceAlert {
    pub id: Option<i64>,
    pub user_id: i64,
    pub symbol: String,
    pub target_price: f64,
    pub condition: AlertCondition,
    pub created_at: i64,
    pub triggered: bool,
}

impl PriceAlert {
    pub fn new(id: Option<i64>, user_id: i64, symbol: String, target_price: f64, condition: AlertCondition) -> Self {
        Self {
            id,
            user_id,
            symbol,
            target_price,
            condition,
            created_at: chrono::Utc::now().timestamp(),
            triggered: false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ApiKey {
    pub id: i64,
    pub user_id: i64,
    pub key: String,
    pub created_at: i64,
    pub last_used: Option<i64>,
    pub expires_at: Option<i64>,
    pub is_active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AlertCondition {
    Above,
    Below,
}

impl FromStr for AlertCondition {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "above" => Ok(AlertCondition::Above),
            "below" => Ok(AlertCondition::Below),
            _ => Err(format!("Invalid condition: {}", s))
        }
    }
}

impl fmt::Display for AlertCondition {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AlertCondition::Above => write!(f, "above"),
            AlertCondition::Below => write!(f, "below"),
        }
    }
}

impl FromSql for AlertCondition {
    fn column_result(value: ValueRef<'_>) -> FromSqlResult<Self> {
        let text = value.as_str()?.to_lowercase();
        match text.as_str() {
            "above" => Ok(AlertCondition::Above),
            "below" => Ok(AlertCondition::Below),
            _ => Err(rusqlite::types::FromSqlError::InvalidType),
        }
    }
}

impl ToSql for AlertCondition {
    fn to_sql(&self) -> SqliteResult<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[derive(Debug, Clone)]
pub struct CryptoPrice {
    pub symbol: String,
    pub price: f64,
    pub exchange: String,
    pub timestamp: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum UserState {
    Idle,
    CreatingPriceAlert {
        step: PriceAlertStep,
        symbol: Option<String>,
        target_price: Option<f64>,
        condition: Option<AlertCondition>,
    },
    CreatingDepegAlert {
        step: DepegAlertStep,
        symbol: Option<String>,
        target_price: Option<f64>,
        differential: Option<f64>,
        exchanges: Option<Vec<String>>,
    },
    CreatingPairAlert {
        step: PairAlertStep,
        token1: Option<String>,
        token2: Option<String>,
        expected_ratio: Option<f64>,
        differential: Option<f64>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PriceAlertStep {
    SelectSymbol,
    EnterPrice,
    SelectCondition,
    Confirm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DepegAlertStep {
    SelectSymbol,
    EnterTargetPrice,
    EnterDifferential,
    SelectExchanges,
    Confirm,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PairAlertStep {
    SelectToken1,
    SelectToken2,
    EnterRatio,
    EnterDifferential,
    Confirm,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq)]
pub enum OrderType {
    Market,
    Limit,
    StopLoss,
    StopLossLimit,
    TakeProfit,
    TakeProfitLimit,
}

#[derive(Debug, Deserialize)]
pub struct OrderRequest {
    pub exchange: String,
    pub symbol: String,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub quantity: Decimal,
    pub price: Option<Decimal>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Order {
    pub id: String,
    pub symbol: String,
    pub order_type: OrderType,
    pub side: OrderSide,
    pub price: Option<Decimal>,
    pub quantity: Decimal,
    pub filled_quantity: Decimal,
    pub status: OrderStatus,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderStatus {
    New,
    PartiallyFilled,
    Filled,
    Canceled,
    Rejected,
    Expired,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Balance {
    pub asset: String,
    pub free: Decimal,
    pub locked: Decimal,
} 