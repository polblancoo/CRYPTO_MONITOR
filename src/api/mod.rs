use axum::{Router, Extension};
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use crate::Database;
use tokio::net::TcpListener;

mod routes;
mod handlers;
mod middleware;
mod extractors;

pub use middleware::logging;

#[derive(Clone)]
pub struct ApiState {
    db: Arc<Database>,
}

pub async fn start_server(db: Arc<Database>, port: u16) -> Result<(), Box<dyn std::error::Error>> {
    // Configurar el estado compartido
    let state = ApiState { db: db.clone() };

    // Configurar CORS
    let cors = CorsLayer::permissive();

    // Crear el router
    let app = Router::new()
        .merge(routes::api_routes())
        .layer(Extension(state))
        .layer(cors);

    // Iniciar el servidor
    let addr = std::net::SocketAddr::from(([127, 0, 0, 1], port));
    println!("API escuchando en http://{}", addr);
    
    let listener = TcpListener::bind(addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
} 