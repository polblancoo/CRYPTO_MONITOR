mod routes;
mod handlers;
mod middleware;
mod extractors;

use axum::{
    routing::{get, post, delete},
    Router,
    Extension,
};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use crate::Database;
use tokio::net::TcpListener;

#[derive(Clone)]
pub struct ApiState {
    db: Arc<Database>,
}

pub async fn start_server(db: Database, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Configurar el estado compartido
    let db = Arc::new(db);
    let state = ApiState { db: db.clone() };

    // Configurar CORS
    let cors = CorsLayer::permissive();

    // Crear el router
    let app = Router::new()
        .route("/api/register", post(handlers::register))
        .route("/api/login", post(handlers::login))
        .route("/api/alerts", post(handlers::create_alert))
        .route("/api/alerts", get(handlers::get_user_alerts))
        .route("/api/alerts/:id", delete(handlers::delete_alert))
        .layer(Extension(state))
        .layer(cors);

    // Iniciar el servidor
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    println!("Servidor escuchando en http://{}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
} 