use axum::{
    async_trait,
    extract::FromRequestParts,
    http::request::Parts,
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub struct BearerAuth(pub String);

#[async_trait]
impl<S> FromRequestParts<S> for BearerAuth
where
    S: Send + Sync,
{
    type Rejection = Response;

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        let auth_header = parts
            .headers
            .get("Authorization")
            .and_then(|value| value.to_str().ok())
            .and_then(|value| value.strip_prefix("Bearer "));

        match auth_header {
            Some(token) => Ok(BearerAuth(token.to_string())),
            None => Err(StatusCode::UNAUTHORIZED.into_response()),
        }
    }
} 