use axum::{
    extract::{Json, Path},
    http::StatusCode,
    response::IntoResponse,
    Extension,
};
use serde::{Deserialize, Serialize};
use crate::models::{User, PriceAlert, AlertCondition};
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
pub struct CreateAlertRequest {
    symbol: String,
    target_price: f64,
    condition: AlertCondition,
}

pub async fn create_alert(
    Extension(state): Extension<ApiState>,
    BearerAuth(token): BearerAuth,
    Json(payload): Json<CreateAlertRequest>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&token) {
        Ok(Some(user)) => {
            let alert = PriceAlert {
                id: None,
                user_id: user.id,
                symbol: payload.symbol,
                target_price: payload.target_price,
                condition: payload.condition,
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

// ... continuará con los handlers de alertas ... 