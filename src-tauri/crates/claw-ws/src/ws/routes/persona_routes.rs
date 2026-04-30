// Claw Desktop - 画像路由 - 处理Agent人物画像的WS请求
use axum::{Json, Router, extract::Extension, routing::post};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;
use claw_harness::harness::types::AgentPersona;

/// 画像路由 — 处理Agent画像的CRUD WS请求
pub struct PersonaRoutes;

/// 获取画像 — 按agent_id查询画像
pub async fn persona_get(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };

    log::info!("[PersonaRoutes:get] agent_id={}", agent_id);

    let mut mgr = state.persona_manager.lock().await;
    match mgr.get_persona(&agent_id) {
        Some(p) => {
            log::info!("[PersonaRoutes:get] Success | agent_id={}", agent_id);
            Json(ApiResponse::ok(serde_json::json!(p)))
        }
        None => {
            log::warn!("[PersonaRoutes:get] Not found | agent_id={}", agent_id);
            Json(ApiResponse::err(&format!(
                "Persona '{}' not found",
                agent_id
            )))
        }
    }
}

/// 保存画像 — 创建或更新完整画像
pub async fn persona_save(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let persona: AgentPersona = match serde_json::from_value(params.clone()) {
        Ok(p) => p,
        Err(e) => {
            log::error!("[PersonaRoutes:save] Invalid persona data: {}", e);
            return Json(ApiResponse::err(&format!("Invalid persona data: {}", e)));
        }
    };

    log::info!("[PersonaRoutes:save] agent_id={}", persona.agent_id);

    let mut mgr = state.persona_manager.lock().await;
    match mgr.save_persona(&persona) {
        Ok(_) => {
            log::info!(
                "[PersonaRoutes:save] Success | agent_id={}",
                persona.agent_id
            );
            Json(ApiResponse::ok(serde_json::json!({ "success": true })))
        }
        Err(e) => {
            log::error!("[PersonaRoutes:save] Failed: {}", e);
            Json(ApiResponse::err(&e.to_string()))
        }
    }
}

/// 更新画像字段 — 修改指定Agent画像的某个字段
pub async fn persona_update_field(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };
    let field = match params.get("field").and_then(|v| v.as_str()) {
        Some(f) => f.to_string(),
        None => return Json(ApiResponse::err("Missing field")),
    };
    let value = match params.get("value").and_then(|v| v.as_str()) {
        Some(v) => v.to_string(),
        None => return Json(ApiResponse::err("Missing value")),
    };

    log::info!(
        "[PersonaRoutes:update_field] agent_id={} field={}",
        agent_id,
        field
    );

    let mut mgr = state.persona_manager.lock().await;
    match mgr.update_persona_field(&agent_id, &field, &value) {
        Ok(_) => {
            log::info!(
                "[PersonaRoutes:update_field] Success | agent_id={}",
                agent_id
            );
            Json(ApiResponse::ok(serde_json::json!({ "success": true })))
        }
        Err(e) => {
            log::error!("[PersonaRoutes:update_field] Failed: {}", e);
            Json(ApiResponse::err(&e.to_string()))
        }
    }
}

/// 删除画像 — 按agent_id删除画像
pub async fn persona_delete(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };

    log::info!("[PersonaRoutes:delete] agent_id={}", agent_id);

    let mut mgr = state.persona_manager.lock().await;
    match mgr.delete_persona(&agent_id) {
        Ok(_) => {
            log::info!("[PersonaRoutes:delete] Success | agent_id={}", agent_id);
            Json(ApiResponse::ok(serde_json::json!({ "success": true })))
        }
        Err(e) => {
            log::error!("[PersonaRoutes:delete] Failed: {}", e);
            Json(ApiResponse::err(&e.to_string()))
        }
    }
}

/// 列出所有画像 — 返回所有Agent画像列表
pub async fn persona_list(
    Extension(state): Extension<Arc<AppState>>,
    Json(_params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    log::info!("[PersonaRoutes:list] Received request");

    let mgr = state.persona_manager.lock().await;
    let personas = mgr.list_personas();
    let count = personas.len();
    log::info!("[PersonaRoutes:list] Success | count={}", count);
    Json(ApiResponse::ok(
        serde_json::json!({ "count": count, "personas": personas }),
    ))
}

/// 构建画像提示词 — 将画像信息融合到系统提示词中
pub async fn persona_build_prompt(
    Extension(state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = match params.get("agent_id").and_then(|v| v.as_str()) {
        Some(id) => id.to_string(),
        None => return Json(ApiResponse::err("Missing agent_id")),
    };
    let base_prompt = params
        .get("base_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    log::info!("[PersonaRoutes:build_prompt] agent_id={}", agent_id);

    let mut mgr = state.persona_manager.lock().await;
    let prompt = mgr.build_enhanced_system_prompt(&agent_id, &base_prompt);
    log::info!(
        "[PersonaRoutes:build_prompt] Success | agent_id={} prompt_len={}",
        agent_id,
        prompt.len()
    );
    Json(ApiResponse::ok(serde_json::json!({ "prompt": prompt })))
}

impl ClawRouter for PersonaRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/persona/get", post(persona_get))
            .route("/api/persona/save", post(persona_save))
            .route("/api/persona/update-field", post(persona_update_field))
            .route("/api/persona/delete", post(persona_delete))
            .route("/api/persona/list", post(persona_list))
            .route("/api/persona/build-prompt", post(persona_build_prompt))
    }
}
