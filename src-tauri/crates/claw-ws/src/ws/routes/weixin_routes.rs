// Claw Desktop - 微信路由 - 处理微信登录的WS请求
use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 微信路由 — 处理微信二维码登录的WS请求
pub struct WeixinRoutes;

/// 获取微信配置 — 从环境变量读取AppID和AppSecret
fn get_weixin_config(registry: &claw_channel::ChannelRegistry) -> Option<claw_channel::plugins::weixin::ilink_api::ILinkConfig> {
    let cfg_mgr = registry.config_manager()?;
    let accounts = tokio::task::block_in_place(|| {
        tokio::runtime::Handle::current().block_on(cfg_mgr.list_accounts())
    }).ok()?;

    let account = accounts.iter().find(|a| a.channel_id.as_str() == "weixin")?;

    Some(claw_channel::plugins::weixin::ilink_api::ILinkConfig {
        token: account.auth_fields.get("token").cloned().unwrap_or_default(),
        account_id: account.auth_fields.get("account_id").cloned().unwrap_or_default(),
        base_url: account.auth_fields.get("base_url").cloned().unwrap_or_else(|| "https://ilinkai.weixin.qq.com".to_string()),
        cdn_base_url: account.auth_fields.get("cdn_base_url").cloned().unwrap_or_else(|| "https://novac2c.cdn.weixin.qq.com/c2c".to_string()),
    })
}

/// 获取微信登录二维码
pub async fn weixin_qrcode(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_channel::bootstrap::get_registry() {
        Some(registry) => {
            let config = match get_weixin_config(&registry) {
                Some(c) => c,
                None => return Json(ApiResponse::err("No WeChat account configured")),
            };

            let client = claw_channel::plugins::weixin::ilink_api::ILinkClient::new(config);
            match client.get_bot_qrcode().await {
                Ok(qr) => Json(ApiResponse::ok(serde_json::to_value(qr).unwrap_or_default())),
                Err(e) => Json(ApiResponse::err(&e)),
            }
        }
        None => Json(ApiResponse::err("Channel registry not initialized")),
    }
}

/// 查询二维码扫描状态
pub async fn weixin_qr_status(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let qrcode_id = params.get("qrcode_id").and_then(|v| v.as_str()).unwrap_or("");

    match claw_channel::bootstrap::get_registry() {
        Some(registry) => {
            let config = match get_weixin_config(&registry) {
                Some(c) => c,
                None => return Json(ApiResponse::err("No WeChat account configured")),
            };

            let client = claw_channel::plugins::weixin::ilink_api::ILinkClient::new(config);
            match client.get_qrcode_status(qrcode_id).await {
                Ok(status) => Json(ApiResponse::ok(serde_json::to_value(status).unwrap_or_default())),
                Err(e) => Json(ApiResponse::err(&e)),
            }
        }
        None => Json(ApiResponse::err("Channel registry not initialized")),
    }
}

impl ClawRouter for WeixinRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/weixin/qrcode", get(weixin_qrcode))
            .route("/api/weixin/qr-status", post(weixin_qr_status))
    }
}
