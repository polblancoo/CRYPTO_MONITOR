// Por ahora vacío, moveremos las rutas aquí más adelante 

use axum::{
    routing::{get, post, delete},
    Router,
};
use super::handlers;

pub fn api_routes() -> Router {
    Router::new()
        // Rutas de autenticación
        .route("/auth/register", post(handlers::register))
        .route("/auth/login", post(handlers::login))
        .route("/auth/reset-api-key", post(handlers::reset_api_key))
        // Rutas de alertas
        .route("/alerts/price", post(handlers::create_price_alert))
        .route("/alerts/depeg", post(handlers::create_depeg_alert))
        .route("/alerts/pair", post(handlers::create_pair_alert))
        .route("/alerts", get(handlers::get_user_alerts))
        .route("/alerts/:id", delete(handlers::delete_alert))
        .route("/alerts/exchanges", get(handlers::get_supported_exchanges))
        .route("/alerts/symbols", get(handlers::get_supported_symbols))
} 