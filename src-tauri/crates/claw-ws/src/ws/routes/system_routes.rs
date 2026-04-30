// Claw Desktop - 系统路由 - 处理系统级WS请求
use axum::{
    Json, Router,
    extract::{Extension, Path, Query},
    routing::{get, post},
};
use sea_orm::ColumnTrait;
use sea_orm::EntityTrait;
use sea_orm::PaginatorTrait;
use sea_orm::QueryFilter;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router::get_config as get_app_config;
use crate::ws::router_trait::ClawRouter;
use crate::ws::server::get_ws_port;
use claw_config::config::AppConfig;

/// 系统路由 — 处理系统管理/健康检查/数据库的WS请求
pub struct SystemRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 导出数据请求
pub struct ExportDataRequest {
    path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 会话信息查询
pub struct SessionInfoQuery {
    conversation_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 使用统计查询
pub struct UsageStatsQuery {
    agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 用户配置查询
pub struct UserProfileQuery {
    agent_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 压缩阈值查询
pub struct CompactionThresholdQuery {
    model_name: String,
}

/// 导出数据到指定路径
pub async fn export_data_to_path(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<ExportDataRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let convs = match claw_db::database::Database::list_conversations().await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    let mut export_data = serde_json::json!({"conversations": [], "messages": []});
    for conv in convs {
        let msgs = claw_db::database::Database::get_messages(&conv.id)
            .await
            .unwrap_or_default();
        export_data["conversations"]
            .as_array_mut()
            .unwrap_or(&mut Vec::new())
            .push(serde_json::to_value(conv).unwrap_or(serde_json::Value::Null));
        export_data["messages"]
            .as_array_mut()
            .unwrap_or(&mut Vec::new())
            .extend(
                msgs.into_iter()
                    .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                    .collect::<Vec<_>>(),
            );
    }
    match serde_json::to_string_pretty(&export_data) {
        Ok(json_str) => match std::fs::write(&req.path, json_str) {
            Ok(()) => Json(ApiResponse::ok(serde_json::json!({ "path": req.path }))),
            Err(e) => Json(ApiResponse::err(&e.to_string())),
        },
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 导出数据（对话框模式） — 自动选择导出路径并导出
pub async fn export_with_dialog(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let export_dir = claw_config::path_resolver::get_app_root().join("exports");
    let _ = std::fs::create_dir_all(&export_dir);
    let filename = format!(
        "claw_export_{}.json",
        chrono::Utc::now().format("%Y%m%d_%H%M%S")
    );
    let path = export_dir.join(&filename);

    let convs = match claw_db::database::Database::list_conversations().await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    let mut export_data = serde_json::json!({"conversations": [], "messages": []});
    for conv in convs {
        let msgs = claw_db::database::Database::get_messages(&conv.id)
            .await
            .unwrap_or_default();
        export_data["conversations"]
            .as_array_mut()
            .unwrap_or(&mut Vec::new())
            .push(serde_json::to_value(conv).unwrap_or(serde_json::Value::Null));
        export_data["messages"]
            .as_array_mut()
            .unwrap_or(&mut Vec::new())
            .extend(
                msgs.into_iter()
                    .map(|m| serde_json::to_value(m).unwrap_or(serde_json::Value::Null))
                    .collect::<Vec<_>>(),
            );
    }
    match serde_json::to_string_pretty(&export_data) {
        Ok(json_str) => match std::fs::write(&path, json_str) {
            Ok(()) => {
                log::info!("[SystemRoutes:export_with_dialog] Exported to {:?}", path);
                Json(ApiResponse::ok(
                    serde_json::json!({ "success": true, "path": path.to_string_lossy().to_string() }),
                ))
            }
            Err(e) => Json(ApiResponse::err(&e.to_string())),
        },
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 导入数据 — 从JSON文件读取会话和消息并写入数据库
pub async fn import_data(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<ExportDataRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let json_str = match std::fs::read_to_string(&req.path) {
        Ok(s) => s,
        Err(e) => return Json(ApiResponse::err(&format!("Failed to read file: {}", e))),
    };

    let data: serde_json::Value = match serde_json::from_str(&json_str) {
        Ok(v) => v,
        Err(e) => return Json(ApiResponse::err(&format!("Invalid JSON: {}", e))),
    };

    let mut imported_convs = 0u64;
    let mut imported_msgs = 0u64;

    if let Some(convs) = data.get("conversations").and_then(|v| v.as_array()) {
        for conv_val in convs {
            let agent_id = conv_val
                .get("agent_id")
                .and_then(|v| v.as_str())
                .map(String::from);
            if let Ok(_) = claw_db::database::Database::create_conversation(agent_id).await {
                imported_convs += 1;
            }
        }
    }

    if let Some(msgs) = data.get("messages").and_then(|v| v.as_array()) {
        for msg_val in msgs {
            let conv_id = msg_val
                .get("conversation_id")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let role = msg_val
                .get("role")
                .and_then(|v| v.as_str())
                .unwrap_or("user");
            let content = msg_val
                .get("content")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if !conv_id.is_empty() && !content.is_empty() {
                if let Ok(_) = claw_db::database::Database::add_message(
                    conv_id, role, content, None, None, None,
                )
                .await
                {
                    imported_msgs += 1;
                }
            }
        }
    }

    log::info!(
        "[SystemRoutes:import_data] Imported {} conversations, {} messages from {}",
        imported_convs,
        imported_msgs,
        req.path
    );
    Json(ApiResponse::ok(serde_json::json!({
        "success": true,
        "imported_conversations": imported_convs,
        "imported_messages": imported_msgs
    })))
}

/// 获取会话信息
pub async fn get_session_info(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<SessionInfoQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let msgs = match claw_db::database::Database::get_messages(&query.conversation_id).await {
        Ok(m) => m,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    let conv = match claw_db::database::Database::get_conversation(&query.conversation_id).await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    Json(ApiResponse::ok(serde_json::json!({
        "conversationId": query.conversation_id,
        "totalMessages": msgs.len(),
        "createdAt": conv.as_ref().map(|c| c.created_at).unwrap_or(0),
        "updatedAt": conv.as_ref().map(|c| c.updated_at).unwrap_or(0),
    })))
}

/// 获取数据库统计
pub async fn get_db_stats(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::list_conversations().await {
        Ok(convs) => {
            let msg_count: u64 = convs.iter().map(|c| c.message_count).sum();
            Json(ApiResponse::ok(
                serde_json::json!({ "totalConversations": convs.len(), "totalMessages": msg_count }),
            ))
        }
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 获取使用统计
pub async fn get_usage_stats(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<UsageStatsQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let convs = match claw_db::database::Database::list_conversations().await {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&e.to_string())),
    };
    let filtered: Vec<_> = if let Some(ref aid) = query.agent_id {
        convs
            .into_iter()
            .filter(|c| c.agent_id.as_deref() == Some(aid))
            .collect()
    } else {
        convs
    };
    let mut total_input_tokens: i64 = 0;
    let mut total_output_tokens: i64 = 0;
    let mut total_messages: u64 = 0;
    let mut model_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    for conv in &filtered {
        if let Ok(msgs) = claw_db::database::Database::get_messages(&conv.id).await {
            for m in &msgs {
                total_messages += 1;
                if let Some(meta) = &m.metadata {
                    if let Ok(usage) = serde_json::from_str::<serde_json::Value>(meta) {
                        if let Some(in_t) = usage.get("input_tokens").and_then(|v| v.as_i64()) {
                            total_input_tokens += in_t;
                        }
                        if let Some(out_t) = usage.get("output_tokens").and_then(|v| v.as_i64()) {
                            total_output_tokens += out_t;
                        }
                    }
                }
                if let Some(model) = &m.model {
                    *model_counts.entry(model.clone()).or_insert(0) += 1;
                }
            }
        }
    }
    let total_cost: f64 = (total_input_tokens as f64 / 1_000_000.0) * 3.0
        + (total_output_tokens as f64 / 1_000_000.0) * 15.0;
    Json(ApiResponse::ok(serde_json::json!({
        "conversationCount": filtered.len(), "messageCount": total_messages,
        "inputTokens": total_input_tokens, "outputTokens": total_output_tokens,
        "totalTokens": total_input_tokens + total_output_tokens,
        "estimatedCostUsd": total_cost, "modelBreakdown": model_counts,
    })))
}

/// 获取队列统计
pub async fn get_queue_stats(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({
        "queue": { "pending": 0, "active": 0 },
        "semaphore": { "available_permits": 10, "max_concurrent": 10 },
        "timestamp": chrono::Utc::now().to_rfc3339(),
    })))
}

/// 获取系统健康状态
pub async fn get_system_health(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let db_health = match claw_db::db::get_db().await.ping().await {
        Ok(_) => serde_json::json!({ "status": "healthy" }),
        Err(e) => serde_json::json!({ "status": "error", "error": e.to_string() }),
    };
    Json(ApiResponse::ok(serde_json::json!({
        "overall_status": "healthy",
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "components": { "database": db_health }
    })))
}

/// 运行诊断检查
pub async fn run_doctor_check(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = get_app_config().await;
    let mut api_key_ok = !config.model.custom_api_key.is_empty()
        || !config.api.api_key.is_empty()
        || std::env::var("ANTHROPIC_API_KEY").is_ok()
        || std::env::var("OPENAI_API_KEY").is_ok();

    if !api_key_ok {
        let db = claw_db::get_db().await;
        use claw_db::db::agent_entities::agent_configs::Entity as AgentConfigs;
        use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
        match AgentConfigs::find()
            .filter(
                claw_db::db::agent_entities::agent_configs::Column::ConfigKey.eq("agent_model_key"),
            )
            .all(db)
            .await
        {
            Ok(agent_keys) => {
                for ak in agent_keys {
                    if !ak.config_value.is_empty()
                        && ak.config_value != "null"
                        && ak.config_value != "undefined"
                    {
                        api_key_ok = true;
                        break;
                    }
                }
            }
            Err(_) => {}
        }
    }

    Json(ApiResponse::ok(serde_json::json!([
        { "name": "API Key", "status": if api_key_ok { "ok" } else { "error" }, "message": if api_key_ok { "API key configured" } else { "No API key found" } },
        { "name": "Model Config", "status": "ok", "message": format!("{} ({})", config.model.default_model, config.model.provider) },
    ])))
}

/// 测试API连接
pub async fn test_connection(
    Extension(_state): Extension<Arc<AppState>>,
    Json(body): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config_value = body
        .get("config")
        .ok_or_else(|| Json(ApiResponse::err("Missing config parameter")));
    let config_value = match config_value {
        Ok(v) => v,
        Err(e) => return e,
    };

    let mut config: AppConfig = match serde_json::from_value(config_value.clone()) {
        Ok(c) => c,
        Err(e) => return Json(ApiResponse::err(&format!("Invalid config: {}", e))),
    };

    if config.model.api_format.is_empty() || config.model.api_format == "auto" {
        let base_lower = config.get_base_url().to_lowercase();
        if base_lower.contains("anthropic") {
            config.model.api_format = "anthropic".to_string();
            config.model.provider = "anthropic".to_string();
        } else {
            config.model.api_format = "openai".to_string();
            config.model.provider = "openai".to_string();
        }
    }

    match claw_llm::llm::test_llm_connection_detailed(&config).await {
        Ok(data) => Json(ApiResponse::ok(data)),
        Err(e) => Json(ApiResponse::<serde_json::Value>::err(&e)),
    }
}

/// 登出
pub async fn logout(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

/// 清除会话消息
pub async fn clear_conversation_messages(
    Extension(_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::clear_conversation_messages(conversation_id).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 压缩会话请求
pub struct CompactConversationRequest {
    conversation_id: String,
    model_name: Option<String>,
}

/// 压缩会话历史
pub async fn compact_conversation(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<CompactConversationRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let model_name = req
        .model_name
        .unwrap_or_else(|| "claude-sonnet-4".to_string());
    match claw_rag::rag::compact_conversation_if_needed(
        &req.conversation_id,
        None,
        &model_name,
        None,
    )
    .await
    {
        Ok(compacted) => Json(ApiResponse::ok(
            serde_json::json!({ "success": true, "compacted": compacted }),
        )),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// RAG压缩请求
pub struct RagCompactRequest {
    conversation_id: String,
    model_name: Option<String>,
}

/// 压缩RAG记忆
pub async fn rag_compact(
    Extension(_state): Extension<Arc<AppState>>,
    Json(req): Json<RagCompactRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let model_name = req
        .model_name
        .unwrap_or_else(|| "claude-sonnet-4".to_string());
    match claw_rag::rag::compact_conversation_if_needed(
        &req.conversation_id,
        None,
        &model_name,
        None,
    )
    .await
    {
        Ok(compacted) => Json(ApiResponse::ok(
            serde_json::json!({ "compacted": compacted }),
        )),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 取消流式请求
pub async fn cancel_stream(
    Extension(_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    claw_llm::llm::request_cancel(&conversation_id);
    Json(ApiResponse::ok(serde_json::json!({ "success": true })))
}

/// 获取用户配置
pub async fn get_user_profile(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<UserProfileQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_rag::rag::get_user_profile(&query.agent_id).await {
        Ok(profile) => Json(ApiResponse::ok(serde_json::json!(profile))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 获取压缩阈值
pub async fn get_compaction_threshold(
    Extension(_state): Extension<Arc<AppState>>,
    Query(query): Query<CompactionThresholdQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let threshold = claw_rag::rag::calc_compaction_threshold(&query.model_name, None);
    let ctx_window = claw_rag::rag::get_model_context_window(&query.model_name);
    Json(ApiResponse::ok(
        serde_json::json!({ "model": query.model_name, "context_window": ctx_window, "compaction_threshold": threshold }),
    ))
}

/// 压缩所有记忆
pub async fn compact_all_memories(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_rag::rag::compact_all_agents().await {
        Ok(count) => Json(ApiResponse::ok(serde_json::json!({
            "success": true,
            "agents_compacted": count,
            "message": format!("Memory compaction completed for {} agents", count)
        }))),
        Err(e) => Json(ApiResponse::err(&format!(
            "Memory compaction failed: {}",
            e
        ))),
    }
}

/// 获取记忆统计
pub async fn get_memory_stats(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let db = claw_db::db::get_db().await;

    let total: u64 = match claw_db::db::entities::memory_units::Entity::find()
        .count(db)
        .await
    {
        Ok(c) => c,
        Err(_) => 0,
    };

    let tool_memories: u64 = match claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::SourceType.eq("tool_init"))
        .count(db)
        .await
    {
        Ok(c) => c,
        Err(_) => 0,
    };

    let compaction_memories: u64 = match claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::SourceType.eq("compaction"))
        .count(db)
        .await
    {
        Ok(c) => c,
        Err(_) => 0,
    };

    Json(ApiResponse::ok(serde_json::json!({
        "total_memories": total,
        "tool_skill_memories": tool_memories,
        "compaction_memories": compaction_memories,
        "conversation_memories": total.saturating_sub(tool_memories).saturating_sub(compaction_memories),
        "max_per_agent": 500,
        "compaction_trigger_at": 400,
    })))
}

/// 获取WebSocket指标
pub async fn get_ws_metrics(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    Json(ApiResponse::ok(crate::ws::server::get_metrics()))
}

/// 获取WebSocket URL
pub async fn get_ws_url(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match get_ws_port() {
        Some(port) => Json(ApiResponse::ok(
            serde_json::json!({ "url": format!("ws://127.0.0.1:{}", port) }),
        )),
        None => Json(ApiResponse::err("WebSocket server not started")),
    }
}

/// 获取数据库状态
pub async fn get_database_status(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let backend_type = claw_config::config::try_get_config()
        .map(|c| c.database.backend.clone())
        .unwrap_or_else(|| "sqlite".to_string());

    let backend = claw_db::db::backend::DatabaseBackend::from(backend_type.as_str());
    match claw_db::db::backend::BackendInitializer::check_status(&backend).await {
        Ok(status) => Json(ApiResponse::ok(
            serde_json::to_value(status).unwrap_or(serde_json::Value::Null),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 初始化数据库
pub async fn initialize_database(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let backend_type = claw_config::config::try_get_config()
        .map(|c| c.database.backend.clone())
        .unwrap_or_else(|| "sqlite".to_string());

    let backend = claw_db::db::backend::DatabaseBackend::from(backend_type.as_str());
    match claw_db::db::backend::BackendInitializer::initialize(&backend).await {
        Ok(result) => {
            if let Ok(mut config) = claw_config::config::get_config().await.map(|c| c.clone()) {
                config.database.initialized = true;
                let _ = config.save(claw_config::path_resolver::get_app_root());
            }
            Json(ApiResponse::ok(
                serde_json::to_value(result).unwrap_or(serde_json::Value::Null),
            ))
        }
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 测试数据库连接
pub async fn test_database_connection(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    let backend_type = params
        .get("backend")
        .and_then(|v| v.as_str())
        .unwrap_or("sqlite")
        .to_string();

    let backend = claw_db::db::backend::DatabaseBackend::from(backend_type.as_str());
    match claw_db::db::backend::BackendInitializer::test_connection(&backend, &params).await {
        Ok(success) => Json(ApiResponse::ok(serde_json::json!({ "success": success }))),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 获取数据库配置
pub async fn get_database_config(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_config::config::get_config().await {
        Ok(config) => Json(ApiResponse::ok(
            serde_json::to_value(&config.database).unwrap_or(serde_json::Value::Null),
        )),
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 更新数据库配置
pub async fn update_database_config(
    Extension(_state): Extension<Arc<AppState>>,
    Json(params): Json<serde_json::Value>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_config::config::get_config().await {
        Ok(config) => {
            let mut config = config.clone();
            match serde_json::from_value::<claw_config::config::DatabaseSettings>(params) {
                Ok(db_config) => {
                    config.database = db_config;
                    match config.save(claw_config::path_resolver::get_app_root()) {
                        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "saved": true }))),
                        Err(e) => Json(ApiResponse::err(&format!("Save failed: {}", e))),
                    }
                }
                Err(e) => Json(ApiResponse::err(&format!("Invalid database config: {}", e))),
            }
        }
        Err(e) => Json(ApiResponse::err(&e)),
    }
}

/// 检查数据库是否已初始化
pub async fn check_database_initialized(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    let initialized = claw_config::config::try_get_config()
        .map(|c| c.database.initialized)
        .unwrap_or(false);
    Json(ApiResponse::ok(
        serde_json::json!({ "initialized": initialized }),
    ))
}

impl ClawRouter for SystemRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/system/export", post(export_data_to_path))
            .route("/api/system/export-with-dialog", post(export_with_dialog))
            .route("/api/system/import", post(import_data))
            .route("/api/system/session-info", get(get_session_info))
            .route("/api/system/db-stats", get(get_db_stats))
            .route("/api/system/usage-stats", get(get_usage_stats))
            .route("/api/system/queue-stats", get(get_queue_stats))
            .route("/api/system/health", get(get_system_health))
            .route("/api/system/doctor", get(run_doctor_check))
            .route("/api/system/test-connection", post(test_connection))
            .route("/api/system/logout", post(logout))
            .route(
                "/api/conversations/:id/clear",
                post(clear_conversation_messages),
            )
            .route("/api/conversations/compact", post(compact_conversation))
            .route("/api/rag/compact", post(rag_compact))
            .route("/api/conversations/:id/cancel-stream", post(cancel_stream))
            .route("/api/memory/user-profile", get(get_user_profile))
            .route(
                "/api/memory/compaction-threshold",
                get(get_compaction_threshold),
            )
            .route("/api/memory/compact-all", post(compact_all_memories))
            .route("/api/memory/stats", get(get_memory_stats))
            .route("/api/system/metrics", get(get_ws_metrics))
            .route("/api/ws/url", get(get_ws_url))
            .route("/api/database/status", get(get_database_status))
            .route("/api/database/initialize", post(initialize_database))
            .route(
                "/api/database/test-connection",
                post(test_database_connection),
            )
            .route("/api/database/config", get(get_database_config))
            .route("/api/database/config", post(update_database_config))
            .route(
                "/api/database/is-initialized",
                get(check_database_initialized),
            )
    }
}
