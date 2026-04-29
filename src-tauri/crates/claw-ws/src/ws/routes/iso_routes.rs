// Claw Desktop - ISO路由 - 处理隔离Agent的WS请求
use axum::{
    extract::Extension,
    routing::post,
    Json, Router,
};
use std::sync::Arc;

use crate::adapters::tool_adapters as ws_adapters;
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 隔离Agent路由 — 处理独立Agent会话管理的WS请求
pub struct IsoRoutes;

// ISO Agent 基础操作 (已在HTTP_METHODS中)
/// 列出所有隔离Agent
pub async fn iso_agent_list(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_list_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
pub async fn iso_agent_create(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_create_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 获取隔离Agent详情
pub async fn iso_agent_get(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_get_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 重命名隔离Agent
pub async fn iso_agent_rename(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_rename_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 删除隔离Agent
pub async fn iso_agent_delete(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_delete_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 设置隔离Agent配置
pub async fn iso_set_config(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_set_config_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 获取隔离Agent配置
pub async fn iso_get_config(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_get_config_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}

// ISO Agent 扩展操作 (WS原生 → HTTP迁移)
/// 初始化隔离Agent数据库
pub async fn iso_init_agent_db(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_init_agent_db_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 设置隔离Agent工具配置
pub async fn iso_set_tools_config(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_set_tools_config_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 设置隔离Agent技能启用
pub async fn iso_set_skills_enabled(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_set_skills_enabled_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 更新隔离Agent配置
pub async fn iso_agent_update_config(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_agent_update_config_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 创建隔离Agent会话
pub async fn iso_create_session(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_create_session_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 列出隔离Agent会话
pub async fn iso_list_sessions(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_list_sessions_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 索引隔离Agent工作区
pub async fn iso_index_workspace(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_index_workspace_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 列出隔离Agent工作区
pub async fn iso_list_workspace(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_list_workspace_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 清理隔离Agent数据
pub async fn iso_cleanup(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_cleanup_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}
/// 生成隔离Agent提示词
pub async fn iso_generate_prompt(Extension(_state): Extension<Arc<AppState>>, Json(params): Json<serde_json::Value>) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::iso_generate_prompt_ws(&params).await { Ok(d) => Json(ApiResponse::ok(d)), Err(e) => Json(ApiResponse::err(&e)) }
}

impl ClawRouter for IsoRoutes {
    fn router() -> Router {
        Router::new()
            // 基础 CRUD
            .route("/api/iso/agent-list", post(iso_agent_list))
            .route("/api/iso/agent-create", post(iso_agent_create))
            .route("/api/iso/agent-get", post(iso_agent_get))
            .route("/api/iso/agent-rename", post(iso_agent_rename))
            .route("/api/iso/agent-delete", post(iso_agent_delete))
            .route("/api/iso/set-config", post(iso_set_config))
            .route("/api/iso/get-config", post(iso_get_config))
            // 扩展操作
            .route("/api/iso/init-agent-db", post(iso_init_agent_db))
            .route("/api/iso/set-tools-config", post(iso_set_tools_config))
            .route("/api/iso/set-skills-enabled", post(iso_set_skills_enabled))
            .route("/api/iso/agent-update-config", post(iso_agent_update_config))
            .route("/api/iso/create-session", post(iso_create_session))
            .route("/api/iso/list-sessions", post(iso_list_sessions))
            .route("/api/iso/index-workspace", post(iso_index_workspace))
            .route("/api/iso/list-workspace", post(iso_list_workspace))
            .route("/api/iso/cleanup", post(iso_cleanup))
            .route("/api/iso/generate-prompt", post(iso_generate_prompt))
    }
}
