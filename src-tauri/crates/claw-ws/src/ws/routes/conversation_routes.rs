// Claw Desktop - 会话路由 - 处理会话/消息相关的WS请求
use axum::{
    extract::{Extension, Path},
    routing::{delete, get, post, put},
    Json, Router,
    response::{sse::Sse, IntoResponse},
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::ws::app_state::AppState;
use crate::ws::response::ApiResponse;
use crate::ws::router_trait::ClawRouter;

/// 会话路由 — 处理聊天会话和消息的WS请求
pub struct ConversationRoutes;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 创建会话查询参数
pub struct CreateConversationQuery {
    agent_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 发送消息请求
pub struct SendMessageRequest {
    conversation_id: String,
    content: String,
    #[serde(default)]
    images: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 重命名会话请求
pub struct RenameConversationRequest {
    new_title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// RAG搜索查询参数
pub struct RagSearchQuery {
    agent_id: Option<String>,
    query: String,
    #[serde(default = "default_limit")]
    limit: usize,
}

/// 默认返回数量 — 50
fn default_limit() -> usize { 10 }

/// 列出会话
pub async fn list_conversations(
    Extension(_state): Extension<Arc<AppState>>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::list_conversations().await {
        Ok(convs) => Json(ApiResponse::ok(serde_json::json!(convs))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

pub async fn create_conversation(
    Extension(_state): Extension<Arc<AppState>>,
    Json(query): Json<CreateConversationQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::create_conversation(query.agent_id).await {
        Ok(conv) => Json(ApiResponse::ok(serde_json::to_value(conv).unwrap_or(serde_json::Value::Null))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 获取消息列表
pub async fn get_messages(
    Extension(_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::get_messages(&conversation_id).await {
        Ok(msgs) => Json(ApiResponse::ok(serde_json::json!(msgs))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 发送消息 — 同步模式
pub async fn send_message(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let config = state.get_config().await;
    
    let has_api_key = !config.model.custom_api_key.is_empty() || !config.api.api_key.is_empty();
    let has_model = !config.model.default_model.is_empty();
    if !has_api_key || !has_model {
        let missing = if !has_api_key && !has_model { "API Key and model name" }
                       else if !has_api_key { "API Key" }
                       else { "model name" };
        return Json(ApiResponse::err(&format!("Model not configured (missing {}). Please configure in global settings first.", missing)));
    }

    if let Err(e) = claw_db::database::Database::add_message(&req.conversation_id, "user", &req.content, None, None, None).await {
        return Json(ApiResponse::err(&e.to_string()));
    }

    match claw_llm::llm::send_chat_message(&config, &req.conversation_id, &req.content, req.images.as_deref()).await {
        Ok(response) => {
            let result = claw_llm::llm::build_send_message_result(response, &config.model.default_model);

            if let Err(e) = claw_db::database::Database::add_message(&req.conversation_id, "assistant", &result.reply_text, None, result.total_tokens, result.metadata_str).await {
                log::error!("[HTTP:Message] Failed to save assistant message: {}", e);
            }

            Json(ApiResponse::ok(serde_json::json!({
                "text": result.reply_text,
                "usage": result.usage,
                "streamed": false
            })))
        }
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// 发送消息 — 同步模式
/// 发送消息 — 流式模式
pub async fn send_message_streaming(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<SendMessageRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    let mut config = state.get_config().await;

    if let Ok(Some(agent_id)) = claw_db::database::Database::get_conversation_agent_id(&req.conversation_id).await {
        if let Some(url) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_url".into()).await.ok().flatten().filter(|s| !s.is_empty()) { 
            config.model.custom_url = url.clone(); 
            config.api.base_url = url; 
        }
        if let Some(key) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_key".into()).await.ok().flatten().filter(|s| !s.is_empty()) { 
            config.model.custom_api_key = key.clone(); 
            config.api.api_key = key.clone(); 
        }
        if let Some(fmt) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_format".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.provider = fmt.to_lowercase();
            config.model.api_format = fmt.to_lowercase();
        }
        if let Some(model) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_default".into()).await.ok().flatten().filter(|s| !s.is_empty()) { 
            config.model.default_model = model; 
        }
        else if let Some(model) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_name".into()).await.ok().flatten().filter(|s| !s.is_empty()) { 
            config.model.default_model = model; 
        }
        
        if let Ok(Some(agent)) = claw_tools::agent_session::iso_agent_get(agent_id.clone()).await {
            if let Some(ref om) = agent.model_override { if !om.is_empty() { config.model.default_model = om.clone(); } }
        }
    }

    let has_api_key = !config.model.custom_api_key.is_empty() || !config.api.api_key.is_empty();
    let has_model = !config.model.default_model.is_empty();
    if !has_api_key || !has_model {
        let missing = if !has_api_key && !has_model { "API Key and model name" }
                       else if !has_api_key { "API Key" }
                       else { "model name" };
        return Json(ApiResponse::err(&format!("Model not configured (missing {}). Please configure in Agent config panel or global settings first.", missing)));
    }

    if let Err(e) = claw_db::database::Database::add_message(&req.conversation_id, "user", &req.content, None, None, None)
        .await {
        return Json(ApiResponse::err(&e.to_string()));
    }

    let config_clone = config.clone();
    let conv_id = req.conversation_id.clone();
    let content_clone = req.content.clone();
    let images_clone = req.images.clone();

    tokio::spawn(async move {
        let app_handle = crate::ws::server::get_app_handle();
        match app_handle {
            Some(handle) => {
                let images_ref = images_clone.as_deref();
                let result = claw_llm::llm::send_chat_message_streaming(&config_clone, &conv_id, &content_clone, handle, images_ref).await;
                match &result {
                    Ok((text, usage)) => {
                        log::info!("[HTTP:Stream] Completed for conv={}, text_len={}", claw_types::truncate_str_safe(&conv_id, 16), text.len());
                        let (total_tokens, metadata_str) = match usage {
                            Some(u) => {
                                let total = u.input_tokens + u.output_tokens;
                                let meta = serde_json::json!({
                                    "input_tokens": u.input_tokens,
                                    "output_tokens": u.output_tokens,
                                    "cache_read": u.cache_read_tokens.or(u.cache_creation_tokens),
                                    "model": config_clone.model.default_model,
                                    "streamed": true
                                });
                                (Some(total as i32), Some(meta.to_string()))
                            }
                            _ => {
                                let est: i32 = (text.len() / 4) as i32;
                                let meta = Some(serde_json::json!({"model": config_clone.model.default_model, "streamed": true, "estimated": true}).to_string());
                                (Some(est), meta)
                            }
                        };
                        if let Err(e) = claw_db::database::Database::add_message(&conv_id, "assistant", text, None, total_tokens, metadata_str).await {
                            log::error!("[HTTP:Stream] Failed to save assistant message: {}", e);
                        }
                        if !text.is_empty() {
                            let title = if content_clone.len() > 50 { 
                                let safe_end = content_clone.char_indices().take(50).last().map(|(i, _)| i).unwrap_or(0);
                                format!("{}...", &content_clone[..safe_end]) 
                            } else { 
                                content_clone.clone() 
                            };
                            let _ = claw_db::database::Database::rename_conversation(&conv_id, &title).await;
                        }
                    }
                    Err(e) => {
                        let conv_preview: String = conv_id.chars().take(16).collect();
                        log::error!("[HTTP:Stream] Streaming failed for {}: {}", conv_preview, e);
                    }
                }
            }
            None => {
                log::error!("[HTTP:Stream] No app_handle available for streaming");
                crate::ws::server::emit_stream("send_message_streaming", "error", serde_json::json!({
                    "type": "error", "conversation_id": conv_id, "content": "Server not ready"
                }));
            }
        }
    });

    Json(ApiResponse::ok(serde_json::json!({ "streamed": true, "conversation_id": req.conversation_id })))
}

/// 删除会话
pub async fn delete_conversation(
    Extension(_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::delete_conversation(&conversation_id).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

pub async fn rename_conversation(
    Extension(_state): Extension<Arc<AppState>>,
    Path(conversation_id): Path<String>,
    Json(req): Json<RenameConversationRequest>,
) -> Json<ApiResponse<serde_json::Value>> {
    match claw_db::database::Database::rename_conversation(&conversation_id, &req.new_title).await {
        Ok(_) => Json(ApiResponse::ok(serde_json::json!({ "success": true }))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

/// RAG搜索 — 在记忆中检索相关信息
pub async fn rag_search(
    Extension(_state): Extension<Arc<AppState>>,
    Json(query): Json<RagSearchQuery>,
) -> Json<ApiResponse<serde_json::Value>> {
    let agent_id = query.agent_id.unwrap_or_else(|| "default".to_string());
    match claw_rag::rag::hybrid_retrieve(&query.query, &agent_id, None, query.limit).await {
        Ok(results) => Json(ApiResponse::ok(serde_json::json!(results))),
        Err(e) => Json(ApiResponse::err(&e.to_string())),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
/// 聊天流式请求
pub struct ChatStreamRequest {
    conversation_id: String,
    content: String,
}

/// 聊天流 — SSE推送流式Token
pub async fn chat_stream(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<ChatStreamRequest>,
) -> axum::response::Response {
    let mut config = state.get_config().await;

    if let Ok(Some(agent_id)) = claw_db::database::Database::get_conversation_agent_id(&req.conversation_id).await {
        if let Some(url) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_url".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.custom_url = url.clone();
            config.api.base_url = url;
        }
        if let Some(key) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_key".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.custom_api_key = key.clone();
            config.api.api_key = key.clone();
        }
        if let Some(fmt) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_format".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.provider = fmt.to_lowercase();
            config.model.api_format = fmt.to_lowercase();
        }
        if let Some(model) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_default".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.default_model = model;
        }
        else if let Some(model) = claw_tools::agent_session::iso_get_config(agent_id.clone(), "agent_model_name".into()).await.ok().flatten().filter(|s| !s.is_empty()) {
            config.model.default_model = model;
        }

        if let Ok(Some(agent)) = claw_tools::agent_session::iso_agent_get(agent_id.clone()).await {
            if let Some(ref om) = agent.model_override { if !om.is_empty() { config.model.default_model = om.clone(); } }
        }
    }

    let has_api_key = !config.model.custom_api_key.is_empty() || !config.api.api_key.is_empty();
    let has_model = !config.model.default_model.is_empty();
    if !has_api_key || !has_model {
        let missing = if !has_api_key && !has_model { "API Key and model name" }
                       else if !has_api_key { "API Key" }
                       else { "model name" };
        let error_event = serde_json::json!({ "type": "error", "message": format!("Model not configured (missing {})", missing) });
        return Sse::new(futures_util::stream::iter(vec![
            Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(serde_json::to_string(&error_event).unwrap_or_else(|_| "{}".to_string())))
        ])).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
    }

    if let Err(e) = claw_db::database::Database::add_message(&req.conversation_id, "user", &req.content, None, None, None).await {
        let error_event = serde_json::json!({ "type": "error", "message": e.to_string() });
        return Sse::new(futures_util::stream::iter(vec![
            Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(serde_json::to_string(&error_event).unwrap_or_else(|_| "{}".to_string())))
        ])).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
    }

    let (tx, rx) = tokio::sync::mpsc::channel::<serde_json::Value>(256);
    let config_clone = config.clone();
    let conv_id = req.conversation_id.clone();
    let content_clone = req.content.clone();

    let app_handle = match crate::ws::server::get_app_handle() {
        Some(h) => h,
        None => {
            let error_event = serde_json::json!({ "type": "error", "message": "Server not ready" });
            return Sse::new(futures_util::stream::iter(vec![
                Ok::<_, std::convert::Infallible>(axum::response::sse::Event::default().data(serde_json::to_string(&error_event).unwrap_or_else(|_| "{}".to_string())))
            ])).keep_alive(axum::response::sse::KeepAlive::default()).into_response();
        }
    };

    let tx_for_callback = tx.clone();
    let conv_id_for_filter = conv_id.clone();
    claw_llm::llm::set_ws_emit_callback(move |conv_id, _event_type, data| {
        if conv_id == conv_id_for_filter || conv_id.is_empty() {
            let _ = tx_for_callback.try_send(data);
        }
    });

    tokio::spawn(async move {
        let _ = tx.send(serde_json::json!({ "type": "session_start", "conversation_id": conv_id })).await;

        match claw_llm::llm::send_chat_message_streaming(&config_clone, &conv_id, &content_clone, app_handle, None).await {
            Ok((text, usage)) => {
                let (total_tokens, metadata_str) = match &usage {
                    Some(u) => {
                        let total = u.input_tokens + u.output_tokens;
                        let meta = serde_json::json!({
                            "input_tokens": u.input_tokens,
                            "output_tokens": u.output_tokens,
                            "model": config_clone.model.default_model,
                            "streamed": true
                        });
                        (Some(total as i32), Some(meta.to_string()))
                    }
                    _ => {
                        let est: i32 = (text.len() / 4) as i32;
                        let meta = Some(serde_json::json!({"model": config_clone.model.default_model, "streamed": true, "estimated": true}).to_string());
                        (Some(est), meta)
                    }
                };
                if let Err(e) = claw_db::database::Database::add_message(&conv_id, "assistant", &text, None, total_tokens, metadata_str).await {
                    log::error!("[ChatStream] Failed to save message: {}", e);
                }
                if !text.is_empty() {
                    let title = if content_clone.len() > 50 { 
                        let safe_end = content_clone.char_indices().take(50).last().map(|(i, _)| i).unwrap_or(0);
                        format!("{}...", &content_clone[..safe_end]) 
                    } else { 
                        content_clone.clone() 
                    };
                    let _ = claw_db::database::Database::rename_conversation(&conv_id, &title).await;
                }
            }
            Err(e) => {
                let _ = tx.send(serde_json::json!({ "type": "error", "message": e.to_string() })).await;
            }
        }
    });

    use futures_util::StreamExt;
    use tokio_stream::wrappers::ReceiverStream;

    Sse::new(ReceiverStream::new(rx).map(|data| {
        Ok::<_, std::convert::Infallible>(
            axum::response::sse::Event::default()
                .data(serde_json::to_string(&data).unwrap_or_default())
        )
    })).keep_alive(axum::response::sse::KeepAlive::new()).into_response()
}

impl ClawRouter for ConversationRoutes {
    fn router() -> Router {
        Router::new()
            .route("/api/conversations", get(list_conversations))
            .route("/api/conversations", post(create_conversation))
            .route("/api/conversations/:id/messages", get(get_messages))
            .route("/api/conversations/send", post(send_message))
            .route("/api/conversations/streaming", post(send_message_streaming))
            .route("/api/chat/stream", post(chat_stream))
            .route("/api/conversations/:id", delete(delete_conversation))
            .route("/api/conversations/:id/rename", put(rename_conversation))
            .route("/api/rag/search", post(rag_search))
    }
}
