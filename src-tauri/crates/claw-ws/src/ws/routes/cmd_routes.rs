// Claw Desktop - 命令路由 - 处理命令行工具管理的WS请求
use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, post},
};
use std::sync::Arc;

use crate::adapters::tool_adapters as ws_adapters;
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 命令路由 — 处理工具注册/技能加载/扩展管理的WS请求
pub struct CmdRoutes;

/// 列出所有已注册工具
pub async fn cmd_list_all_tools(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_list_all_tools_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 注册新工具
pub async fn cmd_register_tool(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_register_tool_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 注销工具
pub async fn cmd_unregister_tool(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_unregister_tool_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 从目录加载技能
pub async fn cmd_load_skills_from_dir(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_load_skills_from_dir_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 列出已加载技能
pub async fn cmd_list_loaded_skills(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_list_loaded_skills_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 扫描扩展
pub async fn cmd_scan_extensions(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_scan_extensions_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 安装扩展
pub async fn cmd_install_extension(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_install_extension_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 卸载扩展
pub async fn cmd_uninstall_extension(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_uninstall_extension_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for CmdRoutes {
    /// 注册命令路由 — 工具/技能/扩展管理接口
    fn router() -> Router {
        Router::new()
            .route("/api/cmd/tools/list", get(cmd_list_all_tools))
            .route("/api/cmd/tools/register", post(cmd_register_tool))
            .route("/api/cmd/tools/unregister", post(cmd_unregister_tool))
            .route("/api/cmd/skills/load", post(cmd_load_skills_from_dir))
            .route("/api/cmd/skills/list-loaded", get(cmd_list_loaded_skills))
            .route("/api/cmd/extensions/scan", get(cmd_scan_extensions))
            .route("/api/cmd/extensions/install", post(cmd_install_extension))
            .route(
                "/api/cmd/extensions/uninstall",
                post(cmd_uninstall_extension),
            )
    }
}
