// Claw Desktop - 记忆路由 - 处理RAG记忆的WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::{delete, get, post},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::adapters::tool_adapters as ws_adapters;
use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 记忆路由 — 处理RAG记忆存储/检索的WS请求
pub struct MemoryRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 记忆存储请求
pub struct MemoryStoreRequest {
    agent_id: String,
    text: String,
    conversation_id: Option<String>,
    #[serde(default = "default_fact_type")]
    fact_type: String,
    #[serde(default = "default_source_type")]
    source_type: String,
    tags: Option<String>,
}

/// 默认事实类型 — fact
fn default_fact_type() -> String {
    "world".to_string()
}
/// 默认来源类型 — conversation
fn default_source_type() -> String {
    "manual".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 记忆检索查询
pub struct MemoryRetrieveQuery {
    agent_id: String,
    query: String,
    conversation_id: Option<String>,
    #[serde(default = "default_limit")]
    limit: usize,
}

/// 默认返回数量 — 10
fn default_limit() -> usize {
    10
}

/// 存储记忆V2 — 保存事实到RAG系统
pub async fn memory_v2_store(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<MemoryStoreRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_rag::rag::store_enhanced_memory(
        &req.agent_id,
        req.conversation_id.as_deref(),
        &req.text,
        &req.fact_type,
        &req.source_type,
        None,
        req.tags.as_deref(),
    )
    .await
    {
        Ok(id) => Json(ApiResponse::ok(
            serde_json::json!({ "success": true, "id": id }),
        )),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 混合检索记忆V2 — 向量+关键词+图检索
pub async fn memory_v2_hybrid_retrieve(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<MemoryRetrieveQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_rag::rag::hybrid_retrieve(
        &query.query,
        &query.agent_id,
        query.conversation_id.as_deref(),
        query.limit,
    )
    .await
    {
        Ok(results) => Json(ApiResponse::ok(
            serde_json::json!({ "count": results.len(), "results": results }),
        )),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 列出记忆实体V2
pub async fn memory_v2_list_entities(
    Extension(_state): Extension<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::memory_v2_list_entities_ws(&agent_id).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取记忆统计V2
pub async fn memory_v2_stats(
    Extension(_state): Extension<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::memory_v2_stats_ws(&agent_id).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 删除记忆V2
pub async fn memory_v2_delete(
    Extension(_state): Extension<Arc<AppState>>,
    Path(unit_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::memory_v2_delete_ws(&unit_id).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 导出记忆V2
pub async fn memory_v2_export(
    Extension(_state): Extension<Arc<AppState>>,
    Path(agent_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match ws_adapters::memory_v2_export_ws(&agent_id).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

impl ClawRouter for MemoryRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/memory/store", post(memory_v2_store))
            .route("/api/memory/retrieve", post(memory_v2_hybrid_retrieve))
            .route(
                "/api/memory/entities/:agent_id",
                get(memory_v2_list_entities),
            )
            .route("/api/memory/stats/:agent_id", get(memory_v2_stats))
            .route("/api/memory/:unit_id", delete(memory_v2_delete))
            .route("/api/memory/export/:agent_id", get(memory_v2_export))
    }
}
