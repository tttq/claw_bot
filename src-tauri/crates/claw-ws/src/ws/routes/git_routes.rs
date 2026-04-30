// Claw Desktop - Git路由 - 处理Git操作的WS请求
use axum::{Json, Router, extract::Extension, routing::post};
use std::sync::Arc;

use crate::adapters::tool_adapters as ws_adapters;
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// Git路由 — 处理Git操作的WS请求
pub struct GitRoutes;

/// 获取Git状态
pub async fn git_status(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_status_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取Git差异
pub async fn git_diff(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_diff_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// Git提交
pub async fn git_commit(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_commit_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取Git日志
pub async fn git_log(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_log_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 列出Git分支
pub async fn git_branch_list(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_branch_list_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 创建Git分支
pub async fn git_create_branch(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_create_branch_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 切换Git分支
pub async fn git_checkout_branch(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_checkout_branch_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// Git暂存
pub async fn git_stash(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_stash_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// Git恢复暂存
/// Git暂存
pub async fn git_stash_pop(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_stash_pop_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// Git添加文件
pub async fn git_add(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_add_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// Git重置
pub async fn git_reset(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_reset_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 检查是否为Git仓库
pub async fn git_is_repository(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::git_is_repository_ws(&params).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for GitRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/git/status", post(git_status))
            .route("/api/git/diff", post(git_diff))
            .route("/api/git/commit", post(git_commit))
            .route("/api/git/log", post(git_log))
            .route("/api/git/branch-list", post(git_branch_list))
            .route("/api/git/create-branch", post(git_create_branch))
            .route("/api/git/checkout-branch", post(git_checkout_branch))
            .route("/api/git/stash", post(git_stash))
            .route("/api/git/stash-pop", post(git_stash_pop))
            .route("/api/git/add", post(git_add))
            .route("/api/git/reset", post(git_reset))
            .route("/api/git/is-repository", post(git_is_repository))
    }
}
