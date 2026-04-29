// Claw Desktop - 配置路由 - 处理应用配置的WS请求
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

/// 配置路由 — 处理应用配置的读取和保存
pub struct ConfigRoutes;

/// 保存配置请求 — 包含完整应用配置JSON
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveConfigRequest {
    config: serde_json::Value,
}

/// 获取应用配置 — 返回当前完整配置
pub async fn get_config(
    Extension(state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = state.get_config().await;
    match serde_json::to_value(config) {
        Ok(value) => Json(ApiResponse::ok(value)),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 保存应用配置 — 验证并持久化到本地文件
pub async fn save_config(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<SaveConfigRequest>,
) -> (StatusCode, Json<ApiResponse<serde_json::Value>>) {
    let cfg_path = claw_config::path_resolver::config_path();
    log::info!("[HTTP:Config] POST /api/config | target={}", cfg_path.display());
    if let Some(parent) = cfg_path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&format!("Failed to create dir {}: {}", parent.display(), e))));
        }
    }

    match serde_json::from_value::<claw_config::config::AppConfig>(req.config.clone()) {
        Ok(config) => {
            let parent = match cfg_path.parent() {
                Some(p) => p,
                None => return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err("config path has no parent"))),
            };
            if let Err(e) = config.save(parent) {
                return (StatusCode::INTERNAL_SERVER_ERROR, Json(ApiResponse::err(&format!("Save failed (path={}): {}", cfg_path.display(), e))));
            }

            state.set_config(config).await;
            log::info!("[HTTP:Config] Config saved successfully");
            (StatusCode::OK, Json(ApiResponse::ok(serde_json::json!({ "success": true }))))
        }
        Err(e) => (StatusCode::BAD_REQUEST, Json(ApiResponse::err(&format!("Invalid config: {}", e)))),
    }
}

impl ClawRouter for ConfigRoutes {
    /// 注册配置路由 — GET/POST /api/config
    fn router() -> Router {
        Router::new()
            .route("/api/config", get(get_config))
            .route("/api/config", post(save_config))
    }
}
