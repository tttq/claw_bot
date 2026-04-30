// Claw Desktop - Hook路由 - 处理事件钩子的WS请求
use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, post},
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// Hook路由 — 处理Webhook的CRUD WS请求
pub struct HookRoutes;

/// 列出所有Hook
pub async fn hook_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let registry = claw_harness::harness::hooks::HookRegistry::new();
    match registry.load_from_db().await {
        Ok(_) => {
            let hooks = registry.list_hooks().await;
            Json(ApiResponse::ok(
                serde_json::to_value(hooks).unwrap_or_default(),
            ))
        }
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 创建Hook
pub async fn hook_create(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let event_str = params.get("event").and_then(|v| v.as_str()).unwrap_or("");
    let event = match claw_harness::harness::hooks::HookEvent::from_str_value(event_str) {
        Some(e) => e,
        None => {
            return Json(ApiResponse::err(&format!(
                "Invalid hook event: {}",
                event_str
            )));
        }
    };

    let hook = claw_harness::harness::hooks::HookDefinition {
        id: params
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&uuid::Uuid::new_v4().to_string())
            .to_string(),
        name: params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled Hook")
            .to_string(),
        event,
        pattern: params
            .get("pattern")
            .and_then(|v| v.as_str())
            .map(String::from),
        handler_type: params
            .get("handler_type")
            .and_then(|v| v.as_str())
            .unwrap_or("log")
            .to_string(),
        handler_config: params
            .get("handler_config")
            .cloned()
            .unwrap_or(serde_json::Value::Object(Default::default())),
        priority: params.get("priority").and_then(|v| v.as_i64()).unwrap_or(0) as i32,
        enabled: params
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
    };

    let registry = claw_harness::harness::hooks::HookRegistry::new();
    match registry.register(hook).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({"registered": true}))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除Hook
pub async fn hook_delete(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let registry = claw_harness::harness::hooks::HookRegistry::new();
    match registry.unregister(id).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({"deleted": true}))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for HookRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/hooks", get(hook_list))
            .route("/api/hooks", post(hook_create))
            .route("/api/hooks/delete", post(hook_delete))
    }
}
