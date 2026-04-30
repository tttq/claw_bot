// Claw Desktop - 多Agent路由 - 处理多Agent协调的WS请求
use axum::{Json, Router, extract::Extension, routing::post};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::agent_engine::{self, AgentTask};
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;
use crate::ws::server;

/// 多Agent路由 — 处理子Agent执行和协调的WS请求
pub struct MultiAgentRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 执行子Agent请求 — 包含Agent ID、指令和上下文
pub struct ExecuteSubAgentRequest {
    task_id: String,
    agent_id: String,
    prompt: String,
    #[serde(default)]
    conversation_id: String,
    #[serde(default = "default_context")]
    context: serde_json::Value,
}

/// 默认上下文 — 空字符串
fn default_context() -> serde_json::Value {
    serde_json::json!({})
}

/// 执行子Agent — 分配任务给指定Agent
pub async fn execute_sub_agent(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<ExecuteSubAgentRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let task = AgentTask {
        task_id: req.task_id,
        agent_id: req.agent_id,
        prompt: req.prompt,
        conversation_id: req.conversation_id,
        context: req.context,
    };

    log::info!(
        "[HTTP:MultiAgent] execute_sub_agent: task={}, agent={}",
        task.task_id,
        task.agent_id
    );

    match agent_engine::execute_agent_task(task).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 发送协调消息 — Agent间通信
pub async fn coordination_message(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let message = body
        .get("message")
        .cloned()
        .ok_or_else(|| Json(ApiResponse::err("Missing message")));
    let message = match message {
        Ok(m) => m,
        Err(e) => return e,
    };

    let from_agent = message
        .get("fromAgentId")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");
    let to_agent = message
        .get("toAgentId")
        .and_then(|v| v.as_str())
        .unwrap_or("*");
    let content = message
        .get("content")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    log::info!(
        "[Coord] {} -> {} : {}",
        from_agent,
        to_agent,
        claw_types::truncate_str_safe(content, 80)
    );

    let coord_data = serde_json::json!({
        "from": from_agent,
        "to": to_agent,
        "content": content,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    });

    if to_agent == "*" {
        server::emit_subagent_event(
            "coord-broadcast",
            "coordination_message",
            coord_data.clone(),
        );
    } else {
        server::emit_subagent_event(
            &format!("coord-{}", to_agent),
            "coordination_message",
            coord_data,
        );
    }

    Json(ApiResponse::ok(serde_json::json!({
        "status": "delivered",
        "from": from_agent,
        "to": to_agent,
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

impl ClawRouter for MultiAgentRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/multi-agent/execute", post(execute_sub_agent))
            .route("/api/multi-agent/coordination", post(coordination_message))
    }
}
