// Claw Desktop - WS中间件 - 认证检查、日志等中间件
use axum::{
    extract::Request,
    http::{header, StatusCode},
    Json,
    middleware::Next,
    response::{IntoResponse, Response},
};
use crate::ws::auth;

/// 认证中间件 — 检查Bearer Token，跳过/api/auth/和/ws/路径
pub async fn auth_middleware(req: Request, next: Next) -> Response {
    let path = req.uri().path();

    if path.starts_with("/api/auth/") || path.starts_with("/ws/") {
        return next.run(req).await;
    }

    let token = req
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    if token.is_empty() || !auth::is_token_valid(token) {
        return (StatusCode::UNAUTHORIZED, Json(serde_json::json!({
            "success": false,
            "error": "Unauthorized: invalid or expired token"
        }))).into_response();
    }

    next.run(req).await
}
