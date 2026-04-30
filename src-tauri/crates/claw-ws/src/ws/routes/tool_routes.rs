// Claw Desktop - 工具路由 - 处理工具调用的WS请求
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

/// 工具路由 — 处理各类工具调用的WS请求
pub struct ToolRoutes;

// ═══ 基础工具 ═══

/// 读取文件工具
pub async fn tool_read(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::file_read_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 编辑文件工具
pub async fn tool_edit(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::file_edit_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 写入文件工具
pub async fn tool_write(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::file_write_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 执行Shell命令工具
pub async fn tool_bash(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_bash_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 取消Shell命令工具
/// 执行Shell命令工具
pub async fn tool_bash_cancel(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_bash_cancel_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 文件模式匹配工具
pub async fn tool_glob(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_glob_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 内容搜索工具
pub async fn tool_grep(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_grep_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 网页抓取工具
pub async fn tool_web_fetch(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_web_fetch_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 网页搜索工具
pub async fn tool_web_search(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_web_search_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 列出所有工具
pub async fn tool_list_all(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(ws_adapters::list_all_tools_ws().await))
}

// ═══ 任务 ═══

/// 写入待办事项工具
pub async fn tool_todo_write(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_todo_write_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 获取待办事项工具
pub async fn tool_todo_get(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_todo_get_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 创建任务工具
pub async fn tool_task_create(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_task_create_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 列出任务工具
pub async fn tool_task_list(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_task_list_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 获取任务工具
pub async fn tool_task_get(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_task_get_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 更新任务工具
pub async fn tool_task_update(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_task_update_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ═══ 调度 ═══

/// 创建定时任务工具
pub async fn tool_schedule_cron(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_schedule_cron_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 列出定时任务工具
pub async fn tool_schedule_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_schedule_list_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ═══ 环境/代码 ═══

/// 获取环境变量
pub async fn get_env_variables(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::get_env_variables_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 获取环境会话信息
pub async fn get_env_session_info(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::get_env_session_info_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 获取代码变更摘要
pub async fn get_code_changes_summary(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::get_code_changes_summary_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 运行代码审查
pub async fn run_code_review(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::run_code_review_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 切换快速模式
pub async fn toggle_fast_mode(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::toggle_fast_mode_ws().await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ═══ 高级工具 (WS原生 → HTTP迁移) ═══

/// Agent工具
pub async fn tool_agent(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_agent_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 工作流工具
pub async fn tool_workflow(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_workflow_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 技能调用工具
pub async fn tool_skill_fn(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_skill_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 简报工具
pub async fn tool_brief(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_brief_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 配置工具
pub async fn tool_config(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_config_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 笔记本编辑工具
pub async fn tool_notebook_edit(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_notebook_edit_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 询问用户工具
pub async fn tool_ask_user_question(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_ask_user_question_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}
/// 工具搜索
pub async fn tool_tool_search(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::tool_tool_search_ws(&params).await {
        Ok(d) => Json(ApiResponse::ok(d)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

// ═══ 计划模式 ═══

/// 进入计划模式 — 切换当前会话到计划模式
pub async fn tool_enter_plan_mode(
    Extension(state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!("[ToolRoutes:enter_plan_mode] Entering plan mode");
    let mut config = state.get_config().await;
    config.advanced.plan_mode = true;
    state.set_config(config).await;
    Json(ApiResponse::ok(
        serde_json::json!({ "success": true, "mode": "plan" }),
    ))
}
/// 退出计划模式 — 切换当前会话回正常模式
pub async fn tool_exit_plan_mode(
    Extension(state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!("[ToolRoutes:exit_plan_mode] Exiting plan mode");
    let mut config = state.get_config().await;
    config.advanced.plan_mode = false;
    state.set_config(config).await;
    Json(ApiResponse::ok(
        serde_json::json!({ "success": true, "mode": "normal" }),
    ))
}
/// 获取计划状态 — 返回当前是否处于计划模式
pub async fn tool_get_plan_status(
    Extension(state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = state.get_config().await;
    let active = config.advanced.plan_mode;
    Json(ApiResponse::ok(serde_json::json!({ "active": active })))
}

// ═══ 标签管理 ═══

/// 标签存储 — 内存中的标签集合
static TAG_STORE: std::sync::OnceLock<tokio::sync::RwLock<Vec<serde_json::Value>>> =
    std::sync::OnceLock::new();

fn tag_store() -> &'static tokio::sync::RwLock<Vec<serde_json::Value>> {
    TAG_STORE.get_or_init(|| tokio::sync::RwLock::new(Vec::new()))
}

/// 添加标签 — 创建新标签（名称+颜色）
pub async fn tool_tag_add(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let name = match body.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return Json(ApiResponse::err("Missing name")),
    };
    let color = body
        .get("color")
        .and_then(|v| v.as_str())
        .unwrap_or("#6B7280")
        .to_string();

    let store = tag_store();
    let mut tags = store.write().await;
    if tags
        .iter()
        .any(|t| t.get("name").and_then(|v| v.as_str()) == Some(&name))
    {
        return Json(ApiResponse::err(&format!("Tag '{}' already exists", name)));
    }
    tags.push(serde_json::json!({ "name": name, "color": color }));
    log::info!("[ToolRoutes:tag_add] Added tag={}", name);
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

/// 删除标签 — 按名称删除标签
pub async fn tool_tag_delete(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let name = match body.get("name").and_then(|v| v.as_str()) {
        Some(n) => n.to_string(),
        None => return Json(ApiResponse::err("Missing name")),
    };

    let store = tag_store();
    let mut tags = store.write().await;
    let before = tags.len();
    tags.retain(|t| t.get("name").and_then(|v| v.as_str()) != Some(&name));
    let removed = before - tags.len();
    log::info!(
        "[ToolRoutes:tag_delete] Removed {} tag(s) named={}",
        removed,
        name
    );
    Json(ApiResponse::ok(
        serde_json::json!({ "success": true, "removed": removed }),
    ))
}

/// 列出所有标签
pub async fn tool_tag_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let store = tag_store();
    let tags = store.read().await;
    log::info!("[ToolRoutes:tag_list] Returning {} tags", tags.len());
    Json(ApiResponse::ok(serde_json::json!({ "tags": *tags })))
}

impl ClawRouter for ToolRoutes {
    fn router() -> Router {
        Router::new()
            // 基础工具
            .route("/api/tools/read", post(tool_read))
            .route("/api/tools/edit", post(tool_edit))
            .route("/api/tools/write", post(tool_write))
            .route("/api/tools/bash", post(tool_bash))
            .route("/api/tools/bash/cancel", post(tool_bash_cancel))
            .route("/api/tools/glob", post(tool_glob))
            .route("/api/tools/grep", post(tool_grep))
            .route("/api/tools/web-fetch", post(tool_web_fetch))
            .route("/api/tools/web-search", post(tool_web_search))
            .route("/api/tools/list-all", post(tool_list_all))
            // 任务
            .route("/api/tools/todo-write", post(tool_todo_write))
            .route("/api/tools/todo-get", post(tool_todo_get))
            .route("/api/tools/task-create", post(tool_task_create))
            .route("/api/tools/task-list", post(tool_task_list))
            .route("/api/tools/task-get", post(tool_task_get))
            .route("/api/tools/task-update", post(tool_task_update))
            // 调度
            .route("/api/tools/schedule-cron", post(tool_schedule_cron))
            .route("/api/tools/schedule-list", post(tool_schedule_list))
            // 环境/代码
            .route("/api/env/variables", get(get_env_variables))
            .route("/api/env/session-info", get(get_env_session_info))
            .route("/api/code/changes-summary", post(get_code_changes_summary))
            .route("/api/code/review", post(run_code_review))
            .route("/api/toggle-fast-mode", post(toggle_fast_mode))
            // 高级工具
            .route("/api/tools/agent", post(tool_agent))
            .route("/api/tools/workflow", post(tool_workflow))
            .route("/api/tools/skill", post(tool_skill_fn))
            .route("/api/tools/brief", post(tool_brief))
            .route("/api/tools/config", post(tool_config))
            .route("/api/tools/notebook-edit", post(tool_notebook_edit))
            .route("/api/tools/ask-user-question", post(tool_ask_user_question))
            .route("/api/tools/tool-search", post(tool_tool_search))
            // 计划模式
            .route("/api/tools/plan-mode/enter", post(tool_enter_plan_mode))
            .route("/api/tools/plan-mode/exit", post(tool_exit_plan_mode))
            .route("/api/tools/plan-mode/status", get(tool_get_plan_status))
            .route("/api/tools/tag-add", post(tool_tag_add))
            .route("/api/tools/tag-delete", post(tool_tag_delete))
            .route("/api/tools/tag-list", post(tool_tag_list))
    }
}
