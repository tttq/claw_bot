// Claw Desktop - Harness路由 - 处理错误学习/画像/可观测性的WS请求
use axum::{
    extract::Extension,
    routing::{get, post},
    Json, Router,
};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// Harness路由 — 处理错误学习/画像/跨记忆/可观测性的WS请求
pub struct HarnessRoutes;

/// 检查错误触发器命中 — 递增指定规则的触发计数
pub async fn harness_error_trigger_hit(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let rule_id = match params.get("rule_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing rule_id")),
    };

    log::info!("[HarnessRoutes:error_trigger_hit] rule_id={}", rule_id);

    let engine = state.error_engine.lock().await;
    engine.trigger_rule_hit(&rule_id).await;
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

/// 更新Agent画像 — 修改指定Agent的画像字段
pub async fn harness_persona_update(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let persona_id = match params.get("persona_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing persona_id")),
    };
    let field = match params.get("field").and_then(|v| v.as_str()) {
        Some(f) => f.to_string(),
        None => return Json(ApiResponse::err("Missing field")),
    };
    let value = match params.get("value").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return Json(ApiResponse::err("Missing value")),
    };

    log::info!("[HarnessRoutes:persona_update] persona_id={} field={}", persona_id, field);

    let mut mgr = state.persona_manager.lock().await;
    match mgr.update_persona_field(&persona_id, &field, &value) {
        Ok(_) => {
            log::info!("[HarnessRoutes:persona_update] Success | persona_id={}", persona_id);
            Json(ApiResponse::ok(serde_json::json!({ "success": true })))
        }
        Err(e) => {
            log::error!("[HarnessRoutes:persona_update] Failed: {}", e);
            Json(ApiResponse::err(&e))
        }
    }
}

/// 捕获错误信息 — 将错误事件交给ErrorLearningEngine处理并生成规避规则
pub async fn harness_error_capture(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let error_message = match params.get("error_message").and_then(|v| v.as_str()) {
        Some(msg) => msg.to_string(),
        None => return Json(ApiResponse::err("Missing error_message")),
    };
    let context = params.get("context").and_then(|v| v.as_str()).map(String::from);
    let category_str = params.get("category").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let category = match category_str.to_lowercase().as_str() {
        "api" => claw_harness::harness::types::ErrorCategory::ApiError,
        "tool" => claw_harness::harness::types::ErrorCategory::ToolError,
        "logic" => claw_harness::harness::types::ErrorCategory::LogicError,
        "context" => claw_harness::harness::types::ErrorCategory::ContextError,
        "validation" => claw_harness::harness::types::ErrorCategory::ValidationError,
        _ => claw_harness::harness::types::ErrorCategory::Other,
    };

    log::info!(
        "[HarnessRoutes:error_capture] agent={} category={:?} msg_len={}",
        agent_id, category, error_message.len()
    );

    let engine = state.error_engine.lock().await;
    let rule_id = engine.capture_and_learn(
        &agent_id,
        &category,
        &error_message,
        context.as_deref(),
        None,
    ).await;

    log::info!("[HarnessRoutes:error_capture] Success | rule_id={}", rule_id);
    Json(ApiResponse::ok(serde_json::json!({ "success": true, "rule_id": rule_id })))
}

/// 跨记忆检索 — 从目标Agent的记忆中检索与查询相关的条目
pub async fn harness_cross_memory_retrieve(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let query = match params.get("query").and_then(|v| v.as_str()) {
        Some(q) => q.to_string(),
        None => return Json(ApiResponse::err("Missing query")),
    };
    let agent_id = params.get("agent_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let target_agent_ids: Vec<String> = params.get("target_agent_ids")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(10) as usize;

    log::info!(
        "[HarnessRoutes:cross_memory_retrieve] source={} targets={:?} query_len={}",
        agent_id, target_agent_ids, query.len()
    );

    let request = claw_harness::harness::types::CrossMemoryRequest {
        source_agent_id: agent_id,
        target_agent_ids,
        query: query.clone(),
        context_limit: Some(limit),
        min_visibility: claw_harness::harness::types::MemoryVisibility::default(),
    };

    match claw_harness::harness::cross_memory::CrossMemoryEngine::retrieve(&request).await {
        Ok(results) => {
            let count = results.len();
            log::info!("[HarnessRoutes:cross_memory_retrieve] Success | count={}", count);
            Json(ApiResponse::ok(serde_json::json!({ "count": count, "results": results })))
        }
        Err(e) => {
            log::error!("[HarnessRoutes:cross_memory_retrieve] Failed: {}", e);
            Json(ApiResponse::err(&e))
        }
    }
}

/// 解析跨记忆提及 — 从文本中提取所有@AgentName引用
pub async fn harness_cross_memory_parse_mentions(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let text = match params.get("text").and_then(|v| v.as_str()) {
        Some(t) => t.to_string(),
        None => return Json(ApiResponse::err("Missing text")),
    };

    log::info!("[HarnessRoutes:cross_memory_parse_mentions] text_len={}", text.len());

    let mentions = claw_harness::harness::cross_memory::CrossMemoryEngine::parse_mentions(&text);
    log::info!("[HarnessRoutes:cross_memory_parse_mentions] Success | mentions={}", mentions.len());
    Json(ApiResponse::ok(serde_json::json!({ "mentions": mentions })))
}

/// 构建错误规避提示词段 — 将Agent的活跃规避规则融合为系统提示词片段
pub async fn harness_error_build_prompt_section(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };

    log::info!("[HarnessRoutes:error_build_prompt_section] agent_id={}", agent_id);

    let engine = state.error_engine.lock().await;
    let section = engine.get_rules_for_prompt(&agent_id).await;
    log::info!("[HarnessRoutes:error_build_prompt_section] Success | agent_id={} section_len={}", agent_id, section.len());
    Json(ApiResponse::ok(serde_json::json!({ "section": section })))
}

/// 获取错误规避规则列表 — 返回指定Agent的所有活跃规避规则
pub async fn harness_error_get_rules(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };

    log::info!("[HarnessRoutes:error_get_rules] agent_id={}", agent_id);

    let engine = state.error_engine.lock().await;
    let rules = engine.get_active_rules(&agent_id).await;
    log::info!("[HarnessRoutes:error_get_rules] Success | agent_id={} count={}", agent_id, rules.len());
    Json(ApiResponse::ok(serde_json::json!({ "rules": rules })))
}

/// 获取可观测性统计 — 返回Agent执行统计
pub async fn harness_observability_stats(
    Extension(state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!("[HarnessRoutes:observability_stats] Received request");
    let config = state.get_config().await;
    let default_agent = &config.model.default_model;
    let stats = state.observability.get_agent_stats(default_agent).await;
    log::info!("[HarnessRoutes:observability_stats] Returned stats for agent={}", default_agent);
    Json(ApiResponse::ok(stats))
}

/// 获取可观测性事件 — 按Agent ID和限制数量查询事件列表
pub async fn harness_observability_events(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!("[HarnessRoutes:observability_events] Received request");
    let agent_id = params.get("agent_id").and_then(|v| v.as_str());
    let limit = params.get("limit").and_then(|v| v.as_u64()).unwrap_or(50) as usize;
    let events = state.observability.get_events(agent_id, None, limit).await;
    match serde_json::to_value(&events) {
        Ok(v) => {
            log::info!("[HarnessRoutes:observability_events] Returned {} events", events.len());
            Json(ApiResponse::ok(v))
        }
        Err(e) => {
            log::error!("[HarnessRoutes:observability_events] Serialization failed: {}", e);
            Json(ApiResponse::err(&e.to_string()))
        }
    }
}

impl ClawRouter for HarnessRoutes {
    /// 注册Harness路由 — 错误学习/画像/跨记忆/可观测性接口
    fn router() -> Router {
        Router::new()
            .route("/api/harness/error-trigger-hit", post(harness_error_trigger_hit))
            .route("/api/harness/persona-update", post(harness_persona_update))
            .route("/api/harness/error-capture", post(harness_error_capture))
            .route("/api/harness/cross-memory/retrieve", post(harness_cross_memory_retrieve))
            .route("/api/harness/cross-memory/parse-mentions", post(harness_cross_memory_parse_mentions))
            .route("/api/harness/error-build-prompt-section", post(harness_error_build_prompt_section))
            .route("/api/harness/error-get-rules", post(harness_error_get_rules))
            .route("/api/harness/observability/stats", get(harness_observability_stats))
            .route("/api/harness/observability/events", post(harness_observability_events))
    }
}
