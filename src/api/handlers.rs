use crate::{
    models::{
        User, 
        PriceAlert, 
        AlertCondition,
        Order as ModelOrder,
        OrderSide as ModelOrderSide,
        OrderType as ModelOrderType,
        OrderStatus,
        Balance
    },
    exchanges::{
        self,
        types::{Exchange, OrderSide, OrderType, Order as ExchangeOrder},
        ExchangeType
    },
    Auth,
    crypto_api::CryptoAPI,
};
use super::ApiState;
use super::extractors::BearerAuth;
use axum::{
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::{Deserialize, Serialize};
use serde_json::json;

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
            let alert = PriceAlert::new(
                None,
                user.id,
                payload.symbol,
                payload.target_price,
                payload.condition,
            );

            match state.db.create_price_alert(alert).await {
                Ok(id) => (
                    StatusCode::CREATED,
                    Json(json!({
                        "id": id,
                        "message": "Alert created successfully"
                    }))
                ).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to create alert: {}", e)
                    }))
                ).into_response(),
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
            let alert = PriceAlert::new(
                None,
                user.id,
                payload.symbol,
                payload.target_price,
                AlertCondition::Above, // o Below según el caso
            );

            match state.db.create_price_alert(alert).await {
                Ok(id) => (
                    StatusCode::CREATED,
                    Json(json!({
                        "id": id,
                        "message": "Alert created successfully"
                    }))
                ).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to create alert: {}", e)
                    }))
                ).into_response(),
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
            let alert = PriceAlert::new(
                None,
                user.id,
                format!("{}/{}", payload.token1, payload.token2),
                payload.expected_ratio,
                AlertCondition::Above,
            );

            match state.db.create_price_alert(alert).await {
                Ok(id) => (
                    StatusCode::CREATED,
                    Json(json!({
                        "id": id,
                        "message": "Alert created successfully"
                    }))
                ).into_response(),
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": format!("Failed to create alert: {}", e)
                    }))
                ).into_response(),
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
                Ok(alerts) => Json::<Vec<PriceAlert>>(alerts).into_response(),
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

#[axum::debug_handler]
pub async fn place_order(
    State(state): State<ApiState>,
    auth: BearerAuth,
    Json(order): Json<ModelOrder>,
) -> Result<Json<ModelOrder>, StatusCode> {
    match state.db.verify_api_key(&auth.0).await {
        Ok(Some(_user)) => {
            match state.exchange_manager.place_order(
                &order.symbol,
                convert_model_side_to_exchange(order.side),
                convert_model_type_to_exchange(order.order_type),
                order.quantity,
                order.price,
            ).await {
                Ok(exchange_order) => Ok(Json(convert_exchange_order_to_model(exchange_order))),
                Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
            }
        }
        Ok(None) => Err(StatusCode::UNAUTHORIZED),
        Err(_) => Err(StatusCode::INTERNAL_SERVER_ERROR),
    }
}

fn convert_model_side_to_exchange(side: ModelOrderSide) -> OrderSide {
    match side {
        ModelOrderSide::Buy => OrderSide::Buy,
        ModelOrderSide::Sell => OrderSide::Sell,
    }
}

fn convert_model_type_to_exchange(order_type: ModelOrderType) -> OrderType {
    match order_type {
        ModelOrderType::Market => OrderType::Market,
        ModelOrderType::Limit => OrderType::Limit,
        ModelOrderType::StopLoss => OrderType::StopLoss,
        ModelOrderType::StopLossLimit => OrderType::StopLossLimit,
        ModelOrderType::TakeProfit => OrderType::TakeProfit,
        ModelOrderType::TakeProfitLimit => OrderType::TakeProfitLimit,
    }
}

fn convert_exchange_order_to_model(order: ExchangeOrder) -> ModelOrder {
    use crate::models::{OrderType as ModelOrderType, OrderSide as ModelOrderSide, OrderStatus as ModelOrderStatus};
    use crate::exchanges::types::{OrderType, OrderSide, OrderStatus};
    
    ModelOrder {
        id: order.id,
        symbol: order.symbol,
        order_type: match order.order_type {
            OrderType::Market => ModelOrderType::Market,
            OrderType::Limit => ModelOrderType::Limit,
            OrderType::StopLoss => ModelOrderType::StopLoss,
            OrderType::StopLossLimit => ModelOrderType::StopLossLimit,
            OrderType::TakeProfit => ModelOrderType::TakeProfit,
            OrderType::TakeProfitLimit => ModelOrderType::TakeProfitLimit,
        },
        side: match order.side {
            OrderSide::Buy => ModelOrderSide::Buy,
            OrderSide::Sell => ModelOrderSide::Sell,
        },
        price: order.price,
        quantity: order.quantity,
        filled_quantity: order.filled_quantity,
        status: match order.status {
            OrderStatus::New => ModelOrderStatus::New,
            OrderStatus::PartiallyFilled => ModelOrderStatus::PartiallyFilled,
            OrderStatus::Filled => ModelOrderStatus::Filled,
            OrderStatus::Canceled => ModelOrderStatus::Canceled,
            OrderStatus::Rejected => ModelOrderStatus::Rejected,
            OrderStatus::Expired => ModelOrderStatus::Expired,
        },
        created_at: order.created_at,
        updated_at: order.updated_at,
    }
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

    // Obtener el símbolo de la orden antes de cancelarla
    let order = state.exchange_manager
        .get_order("", &order_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    state.exchange_manager
        .cancel_order(ExchangeType::Binance, &order.symbol, &order_id)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(StatusCode::NO_CONTENT)
}

pub async fn get_orders(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> Result<Json<Vec<ModelOrder>>, StatusCode> {
    let user_id = auth.0.parse::<i64>()
        .map_err(|_| StatusCode::BAD_REQUEST)?;

    let _user = state.db.get_user_api_key(user_id)
        .await
        .map_err(|_| StatusCode::UNAUTHORIZED)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let exchange_orders = state.exchange_manager
        .get_open_orders(ExchangeType::Binance)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    // Convertir los tipos usando la función helper
    let model_orders = exchange_orders.into_iter()
        .map(convert_exchange_order_to_model)
        .collect();

    Ok(Json(model_orders))
}

pub async fn get_balance(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> Result<Json<Vec<Balance>>, StatusCode> {
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
        .map(|balance| Balance {
            asset: balance.asset,
            free: balance.free,
            locked: balance.locked,
        })
        .collect();

    Ok(Json(model_balances))
}

#[allow(dead_code)]
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

#[allow(dead_code)]
pub async fn create_alert(
    State(state): State<ApiState>,
    auth: BearerAuth,
    Json(mut alert): Json<PriceAlert>,
) -> impl IntoResponse {
    match state.db.verify_api_key(&auth.0).await {
        Ok(Some(user)) => {
            alert.user_id = user.id;
            alert.created_at = chrono::Utc::now().timestamp();
            alert.triggered = false;
            
            match state.db.save_alert(alert).await {
                Ok(_) => StatusCode::CREATED.into_response(),
                Err(e) => {
                    tracing::error!("Error al guardar alerta: {}", e);
                    StatusCode::INTERNAL_SERVER_ERROR.into_response()
                }
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(e) => {
            tracing::error!("Error al verificar API key: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR.into_response()
        }
    }
}

#[allow(dead_code)]
pub async fn get_alerts(
    State(state): State<ApiState>,
    auth: BearerAuth,
) -> impl IntoResponse {
    match state.db.verify_api_key(&auth.0).await {
        Ok(Some(user)) => {
            match state.db.get_user_alerts(user.id).await {
                Ok(alerts) => Json::<Vec<PriceAlert>>(alerts).into_response(),
                Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
            }
        }
        Ok(None) => StatusCode::UNAUTHORIZED.into_response(),
        Err(_) => StatusCode::INTERNAL_SERVER_ERROR.into_response(),
    }
}

pub async fn update_alert(alert: &mut PriceAlert) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    alert.created_at = chrono::Utc::now().timestamp();
    Ok(())
}

// ... continuará con los handlers de alertas ... 