// Claw Desktop - 定时任务路由 - 处理Cron定时任务的WS请求
use axum::{
    Json, Router,
    extract::Extension,
    routing::{get, post},
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 定时任务路由 — 处理Cron任务的CRUD WS请求
pub struct CronRoutes;

/// 列出所有定时任务
pub async fn cron_list(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_harness::harness::cron::CronStore::list().await {
        Ok(jobs) => Json(ApiResponse::ok(
            serde_json::to_value(jobs).unwrap_or_default(),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 创建定时任务
pub async fn cron_create(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let job = claw_harness::harness::cron::CronJob {
        id: params
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or(&uuid::Uuid::new_v4().to_string())
            .to_string(),
        name: params
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("Untitled")
            .to_string(),
        schedule: params
            .get("schedule")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        prompt: params
            .get("prompt")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string(),
        agent_id: params
            .get("agent_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        delivery_channel_id: params
            .get("delivery_channel_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        delivery_chat_id: params
            .get("delivery_chat_id")
            .and_then(|v| v.as_str())
            .map(String::from),
        enabled: params
            .get("enabled")
            .and_then(|v| v.as_bool())
            .unwrap_or(true),
        silent_on_empty: params
            .get("silent_on_empty")
            .and_then(|v| v.as_bool())
            .unwrap_or(false),
        last_run_at: None,
        next_run_at: None,
        run_count: 0,
        last_result: None,
        created_at: chrono::Utc::now().timestamp(),
        updated_at: chrono::Utc::now().timestamp(),
    };
    match claw_harness::harness::cron::CronStore::create(&job).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({"id": job.id}))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 更新定时任务
pub async fn cron_update(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
    if let Ok(Some(mut job)) = claw_harness::harness::cron::CronStore::get(id).await {
        if let Some(name) = params.get("name").and_then(|v| v.as_str()) {
            job.name = name.to_string();
        }
        if let Some(schedule) = params.get("schedule").and_then(|v| v.as_str()) {
            job.schedule = schedule.to_string();
        }
        if let Some(prompt) = params.get("prompt").and_then(|v| v.as_str()) {
            job.prompt = prompt.to_string();
        }
        if let Some(enabled) = params.get("enabled").and_then(|v| v.as_bool()) {
            job.enabled = enabled;
        }
        match claw_harness::harness::cron::CronStore::update(&job).await {
            Ok(_) => Json(ApiResponse::ok(serde_json::json!({"updated": true}))),
            Err(e) => Json(ApiResponse::err(&e)),
        }
    } else {
        Json(ApiResponse::err("Cron job not found"))
    }
}

/// 删除定时任务
pub async fn cron_delete(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
    match claw_harness::harness::cron::CronStore::delete(id).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({"deleted": true}))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 手动触发定时任务
pub async fn cron_trigger(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let id = params.get("id").and_then(|v| v.as_str()).unwrap_or("");
    match claw_harness::harness::cron::CronStore::mark_run(id, Some("Manually triggered")).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({"triggered": true}))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for CronRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/cron", get(cron_list))
            .route("/api/cron", post(cron_create))
            .route("/api/cron/update", post(cron_update))
            .route("/api/cron/delete", post(cron_delete))
            .route("/api/cron/trigger", post(cron_trigger))
    }
}
