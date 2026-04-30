// Claw Desktop - 渠道路由 - 处理消息渠道的WS请求
use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, post},
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::channel_handlers;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 通道路由 — 处理Discord/Telegram等通道管理的WS请求
pub struct ChannelRoutes;

/// 列出所有通道
pub async fn channel_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "1".to_string(),
        msg_type: "request".to_string(),
        method: "channel_list".to_string(),
        params: serde_json::json!({}),
        token: String::new(),
    };
    match channel_handlers::handle_channel_list(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取通道状态
pub async fn channel_status(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "2".to_string(),
        msg_type: "request".to_string(),
        method: "channel_status".to_string(),
        params: serde_json::json!({}),
        token: String::new(),
    };
    match channel_handlers::handle_channel_status(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 创建通道账户
pub async fn channel_create_account(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "3".to_string(),
        msg_type: "request".to_string(),
        method: "channel_create_account".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_create_account(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 更新通道账户
pub async fn channel_update_account(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "4".to_string(),
        msg_type: "request".to_string(),
        method: "channel_update_account".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_update_account(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除通道账户
pub async fn channel_delete_account(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "5".to_string(),
        msg_type: "request".to_string(),
        method: "channel_delete_account".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_delete_account(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 切换通道启用/禁用
pub async fn channel_toggle(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "6".to_string(),
        msg_type: "request".to_string(),
        method: "channel_toggle".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_toggle(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 测试通道连接
pub async fn channel_test_connection(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "7".to_string(),
        msg_type: "request".to_string(),
        method: "channel_test_connection".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_test_connection(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 通过通道发送消息
pub async fn channel_send_message(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "8".to_string(),
        msg_type: "request".to_string(),
        method: "channel_send_message".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_send_message(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取通道配置Schema
pub async fn channel_get_schema(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let req = crate::ws::protocol::WsRequest {
        id: "9".to_string(),
        msg_type: "request".to_string(),
        method: "channel_get_schema".to_string(),
        params,
        token: String::new(),
    };
    match channel_handlers::handle_channel_get_schema(&req).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for ChannelRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/channels", get(channel_list))
            .route("/api/channels/status", get(channel_status))
            .route("/api/channels/account", post(channel_create_account))
            .route("/api/channels/account/update", post(channel_update_account))
            .route("/api/channels/account/delete", post(channel_delete_account))
            .route("/api/channels/toggle", post(channel_toggle))
            .route("/api/channels/test", post(channel_test_connection))
            .route("/api/channels/send-message", post(channel_send_message))
            .route("/api/channels/schema", post(channel_get_schema))
    }
}
