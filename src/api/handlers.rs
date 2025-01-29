use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    Extension,
};
use serde::{Deserialize, Serialize};
use serde_json::json;
use crate::{
    models::{User, PriceAlert, AlertType, AlertCondition},
    crypto_api::CryptoAPI,
};
use crate::Auth;
use super::ApiState;
use super::extractors::BearerAuth;

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
    Extension(state): Extension<ApiState>,
    Json(payload): Json<RegisterRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(state.db.as_ref());
    match auth.register_user(&payload.username, &payload.password) {
        Ok(user) => {
            // Crear API key para el nuevo usuario
            match state.db.create_api_key(user.id) {
                Ok(api_key) => {
                    let response = RegisterResponse {
                        user,
                        api_key: api_key.key,
                    };
                    (StatusCode::CREATED, Json(response)).into_response()
                }
                Err(_) => {
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
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

pub async fn login(
    Extension(state): Extension<ApiState>,
    Json(payload): Json<LoginRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(state.db.as_ref());
    match auth.login(&payload.username, &payload.password) {
        Ok(Some(user)) => {
            // Generar nueva API key
            match state.db.create_api_key(user.id) {
                Ok(api_key) => {
                    let response = RegisterResponse {
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
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreatePriceAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
        Ok(Some(user)) => {
            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: payload.symbol,
                alert_type: AlertType::Price {
                    target_price: payload.target_price,
                    condition: payload.condition,
                },
                created_at: chrono::Utc::now().timestamp(),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(&alert) {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn create_depeg_alert(
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreateDepegAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
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
                created_at: chrono::Utc::now().timestamp(),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(&alert) {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn create_pair_alert(
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreatePairAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
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
                created_at: chrono::Utc::now().timestamp(),
                triggered_at: None,
                is_active: true,
            };

            match state.db.save_alert(&alert) {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn get_user_alerts(
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
        Ok(Some(user)) => {
            match state.db.get_user_alerts(user.id) {
                Ok(alerts) => Json(alerts).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn delete_alert(
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
    Path(alert_id): Path<i64>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
        Ok(Some(user)) => {
            match state.db.get_alert(alert_id) {
                Ok(Some(alert)) if alert.user_id == user.id => {
                    match state.db.delete_alert(alert_id) {
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
    Extension(state): Extension<ApiState>,
    Json(payload): Json<ResetApiKeyRequest>,
) -> impl IntoResponse {
    let auth = Auth::new(state.db.as_ref());
    
    match auth.login(&payload.username, &payload.password) {
        Ok(Some(user)) => {
            match state.db.create_api_key(user.id) {
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
    Extension(_state): Extension<ApiState>,
    BearerAuth(_): BearerAuth,
) -> impl IntoResponse {
    let api = CryptoAPI::new(std::env::var("COINGECKO_API_KEY").unwrap_or_default());
    Json(api.supported_exchanges())
}

pub async fn get_supported_symbols(
    Extension(_state): Extension<ApiState>,
    BearerAuth(_): BearerAuth,
) -> impl IntoResponse {
    let api = CryptoAPI::new(std::env::var("COINGECKO_API_KEY").unwrap_or_default());
    Json(api.supported_symbols())
}

// ... continuará con los handlers de alertas ... 