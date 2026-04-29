// Claw Desktop - 认证路由 - 处理认证握手和公钥获取的WS请求
use axum::{
    extract::Extension,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 认证路由 — 处理RSA握手和Token验证
pub struct AuthRoutes;

/// 握手请求 — 包含RSA加密的会话密钥
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeRequest {
    encrypted_session_key: String,
}

/// 握手响应 — 返回JWT Token和过期时间
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HandshakeResponse {
    token: String,
    expires_at: i64,
}

/// Token验证请求
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidateRequest {
    token: String,
}

/// 获取RSA公钥 — 返回PEM格式公钥供客户端加密
pub async fn get_public_key(
    Extension(_state): Extension<Arc<AppState>>,
) -> (StatusCode, Json<ApiResponse<String>>) {
    match crate::ws::auth::get_public_key_pem() {
        Ok(key) => (StatusCode::OK, Json(ApiResponse::ok(key))),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&e.to_string()))),
    }
}

/// RSA握手 — 解密会话密钥并返回JWT Token
pub async fn handshake(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<HandshakeRequest>,
) -> (StatusCode, Json<ApiResponse<HandshakeResponse>>) {
    match crate::ws::auth::handshake(&body.encrypted_session_key) {
        Ok((token, expires_at)) => (StatusCode::OK, Json(ApiResponse::ok(HandshakeResponse { token, expires_at }))),
        Err(e) => (StatusCode::UNAUTHORIZED, Json(ApiResponse::err(&e.to_string()))),
    }
}

/// 验证Token — 检查JWT Token有效性并返回声明
pub async fn validate_token(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<ValidateRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    match crate::ws::auth::validate_token(&body.token) {
        Ok(claims) => (StatusCode::OK, Json(ApiResponse::ok(serde_json::json!({
            "valid": true,
            "client_id": claims.client_id,
            "expires_at": claims.exp
        })))),
        Err(e) => (StatusCode::UNAUTHORIZED, Json(ApiResponse::err(&e.to_string()))),
    }
}

impl ClawRouter for AuthRoutes {
    /// 注册认证路由 — 公钥获取/握手/验证
    fn router() -> Router {
        Router::new()
            .route("/api/auth/public-key", get(get_public_key))
            .route("/api/auth/handshake", post(handshake))
            .route("/api/auth/validate", post(validate_token))
    }
}
