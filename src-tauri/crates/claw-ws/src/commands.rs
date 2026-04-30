// Claw Desktop - WS命令 - Tauri命令注册（获取WS URL等）

use crate::ws::auth;
use serde_json::json;

/// 获取服务器RSA公钥 — 用于客户端加密会话密钥
#[tauri::command]
pub fn get_server_public_key() -> Result<String, String> {
    auth::get_public_key_pem()
}

/// RSA握手认证 — 解密会话密钥并生成JWT令牌
#[tauri::command]
pub fn auth_handshake(encrypted_session_key: String) -> Result<serde_json::Value, String> {
    let (token, expires_at) = auth::handshake(&encrypted_session_key)?;
    Ok(json!({
        "token": token,
        "expires_at": expires_at,
    }))
}

/// 验证JWT令牌 — 返回令牌有效性和客户端信息
#[tauri::command]
pub async fn auth_validate(token: String) -> Result<serde_json::Value, String> {
    let claims = auth::validate_token(&token)?;
    Ok(json!({
        "valid": true,
        "client_id": claims.client_id,
        "expires_at": claims.exp,
    }))
}

/// 获取WebSocket连接URL
#[tauri::command]
pub fn get_ws_url() -> Result<String, String> {
    match crate::ws::server::get_ws_port() {
        Some(port) => Ok(format!("ws://127.0.0.1:{}", port)),
        None => Err("WebSocket server not started".to_string()),
    }
}
