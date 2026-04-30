// Claw Desktop - 文件系统技能路由 - 处理磁盘技能文件的WS请求
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
use claw_tools::skill_loader;

/// 文件系统技能路由 — 处理技能文件扫描/添加/删除的WS请求
pub struct FsSkillRoutes;

/// 扫描技能目录
pub async fn fs_skill_scan(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let skills = skill_loader::discover_and_load_all_skills().await;
    Json(ApiResponse::ok(
        serde_json::json!({ "count": skills.len(), "skills": skills }),
    ))
}

/// 列出技能文件
pub async fn fs_skill_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let skills = skill_loader::discover_and_load_all_skills().await;
    Json(ApiResponse::ok(
        serde_json::json!({ "count": skills.len(), "skills": skills }),
    ))
}

/// 添加技能文件
pub async fn fs_skill_add(
    Extension(_state): Extension<Arc<AppState>>,
    Json(_params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::cmd_load_skills_from_dir_ws(&_params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除技能文件
pub async fn fs_skill_remove(
    Extension(_state): Extension<Arc<AppState>>,
    Json(_params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::err("删除技能请使用 Bash 工具"))
}

/// 重新加载技能
pub async fn fs_skill_reload(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let skills = skill_loader::discover_and_load_all_skills().await;
    Json(ApiResponse::ok(
        serde_json::json!({ "success": true, "reloaded": skills.len() }),
    ))
}

/// 读取技能源文件
pub async fn fs_skill_read_source(
    Extension(_state): Extension<Arc<AppState>>,
    Json(_params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::err("请使用 Read 工具直接读取 SKILL.md 文件"))
}

/// 更新技能源文件
pub async fn fs_skill_update_source(
    Extension(_state): Extension<Arc<AppState>>,
    Json(_params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::err(
        "请使用 Write/Edit 工具直接编辑 SKILL.md 文件",
    ))
}

/// 获取技能目录路径
pub async fn fs_skills_dir_path(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let dirs = skill_loader::default_skill_directories();
    Json(ApiResponse::ok(
        serde_json::json!({ "paths": dirs.iter().map(|(p, _)| p.display().to_string()).collect::<Vec<_>>() }),
    ))
}

impl ClawRouter for FsSkillRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/fs-skills/scan", get(fs_skill_scan))
            .route("/api/fs-skills/list", get(fs_skill_list))
            .route("/api/fs-skills/add", post(fs_skill_add))
            .route("/api/fs-skills/remove", post(fs_skill_remove))
            .route("/api/fs-skills/reload", get(fs_skill_reload))
            .route("/api/fs-skills/read-source", post(fs_skill_read_source))
            .route("/api/fs-skills/update-source", post(fs_skill_update_source))
            .route("/api/fs-skills/dir-path", get(fs_skills_dir_path))
    }
}
