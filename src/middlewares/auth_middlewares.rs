use axum::{
    body::Body,
    http::{header::AUTHORIZATION, Request, StatusCode},
    middleware::Next,
    response::IntoResponse,
};
use jsonwebtoken::{decode, DecodingKey, Validation};
use serde::{Deserialize, Serialize};

/// Claims
/// JWT Token Claims
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
}

/// jwt_auth_middleware
/// JWT Authentication Middleware
/// If a token exists, extract claims; if not, just move to the next step
pub async fn jwt_auth_middleware(mut req: Request<Body>, next: Next) -> impl IntoResponse {
    let Some(auth_value) = req.headers().get(AUTHORIZATION) else {
        return (StatusCode::UNAUTHORIZED, "No authorization header").into_response();
    };

    let Ok(auth_str) = auth_value.to_str() else {
        return (StatusCode::UNAUTHORIZED, "Invalid authorization header").into_response();
    };

    let Some(token) = auth_str.strip_prefix("Bearer ") else {
        return (StatusCode::UNAUTHORIZED, "Invalid authorization header").into_response();
    };

    let envs = crate::config::get_environments();
    let token_data = match decode::<Claims>(
        token,
        &DecodingKey::from_secret(envs.jwt_secret.as_bytes()),
        &Validation::default(),
    ) {
        Ok(data) => data,
        Err(_) => return (StatusCode::UNAUTHORIZED, "Invalid token").into_response(),
    };

    req.extensions_mut().insert(token_data.claims);
    next.run(req).await
}
