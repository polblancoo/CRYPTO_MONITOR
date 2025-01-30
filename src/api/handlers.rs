use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{
    models::{self, User, PriceAlert, AlertType, AlertCondition},
    crypto_api::CryptoAPI,
    exchanges::{self, ExchangeType},
    Auth,
};
use super::ApiState;
use super::extractors::BearerAuth;
use std::str::FromStr;

#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub struct RegisterResponse {
    user: User,
    api_key: String,
}

pub async fn register(
    State(state): State<ApiState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(&state.db);
    
    match auth.register_user(&payload.username, &payload.password).await {
        Ok(user) => {
            match state.db.create_api_key(user.id).await {
                Ok(api_key) => {
                    let response = RegisterResponse {
                        user,
                        api_key: api_key.key,
                    };
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Err(_) => StatusCode::BAD_REQUEST.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    username: String,
    password: String,
}

#[derive(Debug, Serialize)]
pub struct LoginResponse {
    user: User,
    api_key: String,
}

pub async fn login(
    State(state): State<ApiState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(&state.db);
    
    match auth.login(&payload.username, &payload.password).await {
        Ok(Some(user)) => {
            match state.db.create_api_key(user.id).await {
                Ok(api_key) => {
                    let response = LoginResponse {
                        user,
                        api_key: api_key.key,
                    };
                    Json(response).into_response()
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct CreatePriceAlertRequest {
    pub symbol: String,
    pub target_price: f64,
    pub condition: AlertCondition,
}

#[derive(Debug, Deserialize)]
pub struct CreateDepegAlertRequest {
    pub symbol: String,
    pub target_price: f64,
    pub differential: f64,
    pub exchanges: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct CreatePairAlertRequest {
    pub token1: String,
    pub token2: String,
    pub expected_ratio: f64,
    pub differential: f64,
}

pub async fn create_price_alert(
    State(state): State<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreatePriceAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token).await {
        Ok(Some(user)) => {
            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: payload.symbol,
                alert_type: AlertType::Price {
                    target_price: payload.target_price,
                    condition: payload.condition,
                },
                created_at: Some(chrono::Utc::now().timestamp()),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(alert).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn create_depeg_alert(
    State(state): State<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreateDepegAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token).await {
        Ok(Some(user)) => {
            let exchanges = payload.exchanges.unwrap_or_else(|| vec![
                "binance".to_string(),
                "coinbase".to_string(),
                "kraken".to_string(),
            ]);

            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: payload.symbol,
                alert_type: AlertType::Depeg {
                    target_price: payload.target_price,
                    differential: payload.differential,
                    exchanges,
                },
                created_at: Some(chrono::Utc::now().timestamp()),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(alert).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn create_pair_alert(
    State(state): State<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreatePairAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token).await {
        Ok(Some(user)) => {
            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: format!("{}/{}", payload.token1, payload.token2),
                alert_type: AlertType::PairDepeg {
                    token1: payload.token1,
                    token2: payload.token2,
                    expected_ratio: payload.expected_ratio,
                    differential: payload.differential,
                },
                created_at: Some(chrono::Utc::now().timestamp()),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(alert).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_user_alerts(
    State(state): State<ApiState>,
    BearerAuth(api_key): BearerAuth,
) -> impl IntoResponse {
    match state.db.verify_api_key(&api_key).await {
        Ok(Some(user)) => {
            match state.db.get_user_alerts(user.id).await {
                Ok(alerts) => Json(alerts).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn delete_alert(
    State(state): State<ApiState>,
    BearerAuth(token): BearerAuth,
    Path(alert_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token).await {
        Ok(Some(user)) => {
            match state.db.get_alert(alert_id).await {
                Ok(Some(alert)) if alert.user_id == user.id => {
                    match state.db.delete_alert(alert_id).await {
                        Ok(_) => StatusCode::NO_CONTENT.into_response(),
                        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
                    }
                }
                Ok(Some(_)) => StatusCode::FORBIDDEN.into_response(),
                Ok(None) => StatusCode::NOT_FOUND.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

#[derive(Debug, Deserialize)]
pub struct ResetApiKeyRequest {
    username: String,
    password: String,
}

pub async fn reset_api_key(
    State(state): State<ApiState>,
    Json(payload): Json<ResetApiKeyRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(state.db.as_ref());
    
    match auth.login(&payload.username, &payload.password).await {
        Ok(Some(user)) => {
            match state.db.create_api_key(user.id).await {
                Ok(api_key) => {
                    let response = json!({
                        "message": "API key regenerada exitosamente",
                        "api_key": api_key.key
                    });
                    (StatusCode::OK, Json(response)).into_response()
                }
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_supported_exchanges(
    State(_state): State<ApiState>,
    BearerAuth(_): BearerAuth,
) -> impl IntoResponse {
    let api = CryptoAPI::new(std::env::var("COINGECKO_API_KEY").unwrap_or_default());
    Json(api.supported_exchanges())
}

pub async fn get_supported_symbols(
    State(_state): State<ApiState>,
    BearerAuth(_): BearerAuth,
) -> impl IntoResponse {
    let api = CryptoAPI::new(std::env::var("COINGECKO_API_KEY").unwrap_or_default());
    Json(api.supported_symbols())
}

pub async fn place_order(
    State(state): State<ApiState>,
    auth: BearerAuth,
    Json(req): Json<models::OrderRequest>,
) -> Result<Json<models::Order>, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _user = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let exchange_type = ExchangeType::from_str(&req.exchange)
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    // Convertir tipos
    let exchange_order = exchanges::OrderRequest {
        symbol: req.symbol,
        side: match req.side {
            models::OrderSide::Buy => exchanges::OrderSide::Buy,
            models::OrderSide::Sell => exchanges::OrderSide::Sell,
        },
        order_type: match req.order_type {
            models::OrderType::Market => exchanges::OrderType::Market,
            models::OrderType::Limit => exchanges::OrderType::Limit,
            models::OrderType::StopLoss => exchanges::OrderType::StopLoss,
            models::OrderType::StopLossLimit => exchanges::OrderType::StopLossLimit,
            models::OrderType::TakeProfit => exchanges::OrderType::TakeProfit,
            models::OrderType::TakeProfitLimit => exchanges::OrderType::TakeProfitLimit,
        },
        quantity: req.quantity,
        price: req.price,
    };

    let order = state.exchange_manager
        .execute_order(exchange_type, exchange_order)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convertir de vuelta a modelo
    let model_order = models::Order {
        id: order.id,
        symbol: order.symbol,
        order_type: match order.order_type {
            exchanges::OrderType::Market => models::OrderType::Market,
            exchanges::OrderType::Limit => models::OrderType::Limit,
            exchanges::OrderType::StopLoss => models::OrderType::StopLoss,
            exchanges::OrderType::StopLossLimit => models::OrderType::StopLossLimit,
            exchanges::OrderType::TakeProfit => models::OrderType::TakeProfit,
            exchanges::OrderType::TakeProfitLimit => models::OrderType::TakeProfitLimit,
        },
        side: match order.side {
            exchanges::OrderSide::Buy => models::OrderSide::Buy,
            exchanges::OrderSide::Sell => models::OrderSide::Sell,
        },
        price: order.price,
        quantity: order.quantity,
        filled_quantity: order.filled_quantity,
        status: match order.status {
            exchanges::OrderStatus::New => models::OrderStatus::New,
            exchanges::OrderStatus::PartiallyFilled => models::OrderStatus::PartiallyFilled,
            exchanges::OrderStatus::Filled => models::OrderStatus::Filled,
            exchanges::OrderStatus::Canceled => models::OrderStatus::Canceled,
            exchanges::OrderStatus::Rejected => models::OrderStatus::Rejected,
            exchanges::OrderStatus::Expired => models::OrderStatus::Expired,
        },
        created_at: order.created_at,
        updated_at: order.updated_at,
    };

    Ok(Json(model_order))
}

pub async fn cancel_order(
    State(state): State<ApiState>,
    auth: BearerAuth,
    Path(order_id): Path<String>,
) -> Result<StatusCode, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _user = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Por ahora asumimos Binance como exchange por defecto
    state.exchange_manager
        .cancel_order(ExchangeType::Binance, &order_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_orders(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> Result<Json<Vec<models::Order>>, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _user = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let exchange_orders = state.exchange_manager
        .get_open_orders(ExchangeType::Binance)  // Por ahora usamos Binance por defecto
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convertir los tipos
    let model_orders = exchange_orders.into_iter()
        .map(|order| models::Order {
            id: order.id,
            symbol: order.symbol,
            order_type: match order.order_type {
                exchanges::OrderType::Market => models::OrderType::Market,
                exchanges::OrderType::Limit => models::OrderType::Limit,
                exchanges::OrderType::StopLoss => models::OrderType::StopLoss,
                exchanges::OrderType::StopLossLimit => models::OrderType::StopLossLimit,
                exchanges::OrderType::TakeProfit => models::OrderType::TakeProfit,
                exchanges::OrderType::TakeProfitLimit => models::OrderType::TakeProfitLimit,
            },
            side: match order.side {
                exchanges::OrderSide::Buy => models::OrderSide::Buy,
                exchanges::OrderSide::Sell => models::OrderSide::Sell,
            },
            price: order.price,
            quantity: order.quantity,
            filled_quantity: order.filled_quantity,
            status: match order.status {
                exchanges::OrderStatus::New => models::OrderStatus::New,
                exchanges::OrderStatus::PartiallyFilled => models::OrderStatus::PartiallyFilled,
                exchanges::OrderStatus::Filled => models::OrderStatus::Filled,
                exchanges::OrderStatus::Canceled => models::OrderStatus::Canceled,
                exchanges::OrderStatus::Rejected => models::OrderStatus::Rejected,
                exchanges::OrderStatus::Expired => models::OrderStatus::Expired,
            },
            created_at: order.created_at,
            updated_at: order.updated_at,
        })
        .collect();

    Ok(Json(model_orders))
}

pub async fn get_balance(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> Result<Json<Vec<models::Balance>>, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _user = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let exchange_balances = state.exchange_manager
        .get_balances()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convertir los tipos
    let model_balances = exchange_balances.into_iter()
        .map(|balance| models::Balance {
            asset: balance.asset,
            free: balance.free,
            locked: balance.locked,
        })
        .collect();

    Ok(Json(model_balances))
}

pub async fn get_user_api_key(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> Result<Json<String>, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let api_key = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    Ok(Json(api_key))
}

pub async fn create_alert(
    State(state): State<ApiState>,
    auth: BearerAuth,
    Json(mut alert): Json<PriceAlert>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&auth.0).await {
        Ok(Some(user)) => {
            alert.user_id = user.id;
            alert.created_at = Some(chrono::Utc::now().timestamp());
            match state.db.save_alert(alert).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_alerts(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> impl IntoResponse {
    match state.db.verify_api_key(&auth.0).await {
        Ok(Some(user)) => {
            match state.db.get_user_alerts(user.id).await {
                Ok(alerts) => Json(alerts).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

// ... continuará con los handlers de alertas ... 