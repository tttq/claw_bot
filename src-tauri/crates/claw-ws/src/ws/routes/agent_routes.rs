// Claw Desktop - Agent路由 - 处理Agent管理的WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path},
    routing::{delete, get, post},
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// Agent路由 — 处理Agent管理的WS请求
pub struct AgentRoutes;

static MGR: std::sync::OnceLock<
    std::sync::Arc<tokio::sync::Mutex<claw_tools::agent_manager::AgentManager>>,
> = std::sync::OnceLock::new();

/// 获取Agent管理器单例 — 首次调用时初始化
fn get_agent_mgr() -> std::sync::Arc<tokio::sync::Mutex<claw_tools::agent_manager::AgentManager>> {
    MGR.get_or_init(|| {
        std::sync::Arc::new(tokio::sync::Mutex::new(
            claw_tools::agent_manager::AgentManager::new(),
        ))
    })
    .clone()
}

/// 列出所有已加载Agent
pub async fn agent_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    let agents = mgr.lock().await.list_loaded();
    Json(ApiResponse::ok(
        serde_json::json!({ "count": agents.len(), "agents": agents }),
    ))
}

/// 创建新Agent — 从JSON定义创建并持久化
pub async fn agent_create(
    Extension(_state): Extension<Arc<AppState>>,
    Json(agent): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_def: claw_tools::agent_manager::AgentDefinition = match serde_json::from_value(agent)
    {
        Ok(a) => a,
        Err(e) => return Json(ApiResponse::err(&format!("Invalid agent data: {}", e))),
    };

    let mgr = get_agent_mgr();
    match mgr.lock().await.create_agent(&agent_def) {
        Ok(dir) => Json(ApiResponse::ok(
            serde_json::json!({ "success": true, "directory": dir }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除Agent — 按ID移除Agent及其文件
pub async fn agent_remove(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.remove_agent(&id) {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取Agent详情 — 按ID查询Agent信息
pub async fn agent_get(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.get_agent(&id) {
        Some(agent) => Json(ApiResponse::ok(serde_json::json!(agent))),
        None => Json(ApiResponse::err(&format!("Agent '{}' not found", id))),
    }
}

impl ClawRouter for AgentRoutes {
    /// 注册Agent路由 — /api/agents CRUD接口
    fn router() -> Router {
        Router::new()
            .route("/api/agents", get(agent_list))
            .route("/api/agents", post(agent_create))
            .route("/api/agents/:id", delete(agent_remove))
            .route("/api/agents/:id", get(agent_get))
    }
}
