// Por ahora vacío, moveremos las rutas aquí más adelante 

use axum::{
    routing::{get, post, delete},
    Router,
};
use super::handlers;
use super::ApiState;

pub fn api_routes() -> Router<ApiState> {
    Router::new()
        // Autenticación
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/reset-api-key", post(handlers::reset_api_key))
        
        // Alertas
        .route("/alerts/price", post(handlers::create_price_alert))
        .route("/alerts/depeg", post(handlers::create_depeg_alert))
        .route("/alerts/pair", post(handlers::create_pair_alert))
        .route("/alerts", get(handlers::get_user_alerts))
        .route("/alerts/:id", delete(handlers::delete_alert))
        
        // Información de mercado
        .route("/exchanges", get(handlers::get_supported_exchanges))
        .route("/symbols", get(handlers::get_supported_symbols))
}

pub fn trading_routes() -> Router<ApiState> {
    Router::new()
        // Trading
        .route("/orders", post(handlers::place_order))
        .route("/orders", get(handlers::get_orders))
        .route("/orders/:order_id", delete(handlers::cancel_order))
        .route("/balance", get(handlers::get_balance))
}

pub fn price_routes() -> Router<ApiState> {
    Router::new()
        .route("/alerts/price", post(handlers::create_price_alert))
        // ... otras rutas ...
} 