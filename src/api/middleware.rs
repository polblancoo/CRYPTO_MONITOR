use axum::{
    body::Body,
    http::Request,
    middleware::Next,
    response::Response,
};

pub async fn logging(
    req: Request<Body>,
    next: Next,
) -> Response {
    let method = req.method().clone();
    let uri = req.uri().clone();
    println!("--> {} {}", method, uri.path());
    
    let response = next.run(req).await;
    println!("<-- {} {}", response.status(), uri.path());
    response
} 