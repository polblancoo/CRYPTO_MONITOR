use axum::Router;
use tower_http::cors::CorsLayer;
use crate::Database;
use crate::exchanges::ExchangeManager;
use tokio::net::TcpListener;
use std::sync::Arc;

mod routes;
mod handlers;
mod middleware;
mod extractors;

pub use middleware::logging;

#[derive(Clone)]
pub struct ApiState {
    pub db: Arc<Database>,
    pub exchange_manager: Arc<ExchangeManager>,
}

pub async fn start_server(
    db: Arc<Database>, 
    exchange_manager: Arc<ExchangeManager>
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let state = ApiState {
        db,
        exchange_manager,
    };

    let cors = CorsLayer::permissive();

    let app = Router::new()
        .nest("/api", routes::api_routes())
        .nest("/trade", routes::trading_routes())
        .with_state(state)
        .layer(cors);

    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], 8080));
    println!("API escuchando en http://{}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
} 