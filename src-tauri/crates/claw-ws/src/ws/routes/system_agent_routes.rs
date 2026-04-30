// Claw Desktop - 系统Agent路由 - 处理系统级Agent的WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path},
    routing::{get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;

static MGR: std::sync::OnceLock<
    std::sync::Arc<tokio::sync::Mutex<claw_tools::agent_manager::AgentManager>>,
> = std::sync::OnceLock::new();

/// 获取Agent管理器单例
fn get_agent_mgr() -> std::sync::Arc<tokio::sync::Mutex<claw_tools::agent_manager::AgentManager>> {
    MGR.get_or_init(|| {
        std::sync::Arc::new(tokio::sync::Mutex::new(
            claw_tools::agent_manager::AgentManager::new(),
        ))
    })
    .clone()
}

/// 列出Agent工作区文件
pub async fn agent_list_workspace(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.list_workspace_files(&id) {
        Ok(files) => Json(ApiResponse::ok(
            serde_json::json!({ "count": files.len(), "files": files }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 写入文件请求 — 包含路径和内容
pub struct WriteFileRequest {
    rel_path: String,
    content: String,
}

/// 写入Agent工作区文件
pub async fn agent_write_file(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<WriteFileRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr
        .lock()
        .await
        .write_workspace_file(&id, &req.rel_path, &req.content)
    {
        Ok(full_path) => Json(ApiResponse::ok(
            serde_json::json!({ "success": true, "path": full_path }),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 读取文件查询 — 包含路径
pub struct ReadFileQuery {
    rel_path: String,
}

/// 读取Agent工作区文件
pub async fn agent_read_file(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ReadFileQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.read_workspace_file(&id, &req.rel_path) {
        Ok(content) => Json(ApiResponse::ok(serde_json::json!({ "content": content }))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除Agent工作区文件
pub async fn agent_delete_file(
    Extension(_state): Extension<Arc<AppState>>,
    Path(id): Path<String>,
    Json(req): Json<ReadFileQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.delete_workspace_file(&id, &req.rel_path) {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 重新加载Agent
pub async fn agent_reload(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    match mgr.lock().await.hot_reload() {
        Ok(result) => Json(ApiResponse::ok(serde_json::json!(result))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取Agent目录路径
pub async fn agents_dir_path(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mgr = get_agent_mgr();
    Json(ApiResponse::ok(
        serde_json::json!({ "path": mgr.lock().await.agents_dir_path() }),
    ))
}

/// 注册系统Agent路由
pub fn router() -> Router {
    Router::new()
        .route("/api/agents/:id/workspace", get(agent_list_workspace))
        .route("/api/agents/:id/write-file", post(agent_write_file))
        .route("/api/agents/:id/read-file", post(agent_read_file))
        .route("/api/agents/:id/delete-file", post(agent_delete_file))
        .route("/api/agents/reload", get(agent_reload))
        .route("/api/agents/dir-path", get(agents_dir_path))
}
