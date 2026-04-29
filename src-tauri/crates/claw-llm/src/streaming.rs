// Claw Desktop - 流式处理 - SSE流式响应的解析和分发
use std::sync::OnceLock;
use tauri::Emitter;
use claw_config::config::AppConfig;
use claw_types::common::ToolDefinition;

static WS_EMIT_CALLBACK: OnceLock<Box<dyn Fn(&str, &str, serde_json::Value) + Send + Sync>> = OnceLock::new();

/// 设置WebSocket事件发射回调 — 用于同时通过WS推送流式事件
pub fn set_ws_emit_callback(callback: impl Fn(&str, &str, serde_json::Value) + Send + Sync + 'static) {
    let _ = WS_EMIT_CALLBACK.set(Box::new(callback));
}

/// 发射聊天流事件 — 同时通过Tauri事件和WS回调推送
pub fn emit_chat_stream(app_handle: &tauri::AppHandle, event_data: serde_json::Value) -> Result<(), tauri::Error> {
    let event_type = event_data.get("type").and_then(|v| v.as_str()).unwrap_or("unknown").to_string();
    let conv_id = event_data.get("conversation_id").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let _ = app_handle.emit("chat-stream", event_data.clone());
    if let Some(cb) = WS_EMIT_CALLBACK.get() {
        cb(&conv_id, &event_type, event_data);
    }
    Ok(())
}

type ApiResponseInner = (String, Vec<crate::llm::ToolCallInfo>, String, Option<crate::llm::UsageInfo>, Option<String>);

pub(crate) use crate::llm::build_api_url;

/// 清理工具名称中的非法字符（冒号/点/空格→下划线），适配API规范
pub fn sanitize_tool_name_for_api(name: &str) -> String {
    name.replace(':', "_").replace('.', "_").replace(' ', "_")
}

/// 还原工具名称 — 将API返回的"Skill_"前缀恢复为"Skill:"
pub fn restore_tool_name_from_api(name: &str) -> String {
    if name.starts_with("Skill_") {
        return name.replacen("Skill_", "Skill:", 1);
    }
    name.to_string()
}

/// OpenAI流式调用 — 解析SSE事件流，提取文本/工具调用/推理内容/用量
pub async fn call_openai_streaming(
    client: &reqwest::Client, base_url: &str, api_key: &str, config: &AppConfig,
    messages: &[serde_json::Value], tools: &[ToolDefinition],
    app_handle: &tauri::AppHandle, conversation_id: &str,
) -> Result<ApiResponseInner, anyhow::Error> {
    let base = base_url.trim_end_matches('/');
    let url = build_api_url(base, "/chat/completions");

    let api_tools: Vec<serde_json::Value> = tools.iter().map(|t| serde_json::json!({
        "type": "function",
        "function": { "name": sanitize_tool_name_for_api(&t.name), "description": t.description, "parameters": t.input_schema }
    })).collect();

    let mut body = serde_json::json!({
        "model": config.model.default_model,
        "max_tokens": config.model.max_tokens,
        "stream": true,
        "messages": messages,
        "tools": api_tools
    });

    if !crate::llm::model_ignores_temperature(&config.model.default_model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(&config.model.default_model, config.model.temperature));
    }
    if !crate::llm::model_ignores_top_p(&config.model.default_model) {
        body["top_p"] = serde_json::json!(crate::llm::effective_top_p(&config.model.default_model, config.model.top_p));
    }
    if crate::llm::model_uses_reasoning_effort(&config.model.default_model) {
        body["reasoning_effort"] = serde_json::json!("medium");
    }
    if let Some(thinking) = crate::llm::build_thinking_param(&config.model.default_model, config.model.thinking_budget) {
        body["thinking"] = thinking;
    }

    let resp = client.post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body).send().await?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("OpenAI API error ({}): {}", status, resp.text().await.unwrap_or_default());
    }

    let mut full_text = String::new();
    let mut reasoning_text = String::new();
    let mut tool_uses: Vec<crate::llm::ToolCallInfo> = Vec::new();
    let mut tool_calls_map: std::collections::HashMap<usize, (String, String, String)> = std::collections::HashMap::new();
    let mut finish_reason = "stop".to_string();
    let mut usage: Option<crate::llm::UsageInfo> = None;

    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;
    let mut sse_buffer = String::with_capacity(4096);
    let mut stream_done = false;

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;

        if crate::llm::is_cancelled(conversation_id) {
            log::info!("[Streaming:OpenAI] Cancelled by user during stream, aborting");
            break;
        }

        sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = sse_buffer.find('\n') {
            let line = sse_buffer.drain(..line_end).collect::<String>();
            if !sse_buffer.is_empty() { sse_buffer.drain(..1); }

            let line = line.trim();
            if !line.starts_with("data: ") { continue; }
            let data = &line.as_bytes()[6..];
            if data == b"[DONE]" { stream_done = true; break; }

            let data_str = std::str::from_utf8(data).unwrap_or("");
            if let Ok(evt) = serde_json::from_str::<serde_json::Value>(data_str) {
                if let Some(choices) = evt["choices"].as_array() {
                    if let Some(choice) = choices.first() {
                        if let Some(fr) = choice["finish_reason"].as_str() {
                            if fr != "null" && !fr.is_empty() { finish_reason = fr.to_string(); }
                        }
                        if let Some(delta) = choice.get("delta") {
                            if let Some(content) = delta["content"].as_str() {
                                if !content.is_empty() {
                                    full_text.push_str(content);
                                    emit_chat_stream(app_handle, serde_json::json!({
                                        "type": "token", "conversation_id": conversation_id, "content": content
                                    })).ok();
                                    tokio::task::yield_now().await;
                                }
                            }
                            if let Some(rc) = delta.get("reasoning_content").and_then(|v| v.as_str()) {
                                if !rc.is_empty() {
                                    reasoning_text.push_str(rc);
                                    emit_chat_stream(app_handle, serde_json::json!({
                                        "type": "thinking", "conversation_id": conversation_id, "content": rc
                                    })).ok();
                                    tokio::task::yield_now().await;
                                }
                            }
                            if let Some(rc) = delta.get("reasoning").and_then(|v| v.as_str()) {
                                if !rc.is_empty() && reasoning_text.is_empty() {
                                    reasoning_text.push_str(rc);
                                    emit_chat_stream(app_handle, serde_json::json!({
                                        "type": "thinking", "conversation_id": conversation_id, "content": rc
                                    })).ok();
                                    tokio::task::yield_now().await;
                                }
                            }
                            if let Some(tcs) = delta["tool_calls"].as_array() {
                                for tc in tcs {
                                    let idx = tc["index"].as_u64().unwrap_or(0) as usize;
                                    let entry = tool_calls_map.entry(idx).or_insert((String::new(), String::new(), String::new()));
                                    if let Some(id) = tc["id"].as_str() { entry.0 = id.to_string(); }
                                    if let Some(name) = tc["function"]["name"].as_str() { entry.1 = restore_tool_name_from_api(name); }
                                    if let Some(args) = tc["function"]["arguments"].as_str() { entry.2.push_str(args); }
                                }
                            }
                        }
                    }
                }
                if let Some(u) = evt.get("usage") {
                    usage = Some(crate::llm::UsageInfo {
                        input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        output_tokens: u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                        cache_read_tokens: None, cache_creation_tokens: None,
                    });
                }
            }
        }

        if stream_done { break; }
    }

    for (_, (id, name, args_str)) in tool_calls_map.into_iter() {
        let input = serde_json::from_str(&args_str).unwrap_or(serde_json::Value::Null);
        tool_uses.push(crate::llm::ToolCallInfo { id, name, input });
    }

    let reasoning_out = if reasoning_text.is_empty() { None } else { Some(reasoning_text) };
    Ok((full_text, tool_uses, finish_reason, usage, reasoning_out))
}

/// Anthropic流式调用 — 解析SSE事件流，提取文本/工具调用/思考内容/用量
pub async fn call_anthropic_streaming(
    client: &reqwest::Client, base_url: &str, api_key: &str, config: &AppConfig,
    messages: &[serde_json::Value], tools: &[ToolDefinition],
    app_handle: &tauri::AppHandle, conversation_id: &str,
) -> Result<ApiResponseInner, anyhow::Error> {
    let base = base_url.trim_end_matches('/');
    let url = build_api_url(base, "/v1/messages");

    let api_tools: Vec<serde_json::Value> = tools.iter().map(|t| serde_json::json!({
        "name": sanitize_tool_name_for_api(&t.name), "description": t.description, "input_schema": t.input_schema
    })).collect();

    let system_prompt = messages.iter()
        .find(|m| m["role"].as_str() == Some("system"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or("");
    let api_messages: Vec<&serde_json::Value> = messages.iter().filter(|m| m["role"].as_str() != Some("system")).collect();

    let mut body = serde_json::json!({
        "model": config.model.default_model,
        "max_tokens": config.model.max_tokens,
        "stream": true,
        "messages": api_messages,
        "tools": api_tools
    });
    if !system_prompt.is_empty() { body["system"] = serde_json::json!(system_prompt); }
    if !crate::llm::model_ignores_temperature(&config.model.default_model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(&config.model.default_model, config.model.temperature));
    }
    if !crate::llm::model_ignores_top_p(&config.model.default_model) {
        body["top_p"] = serde_json::json!(crate::llm::effective_top_p(&config.model.default_model, config.model.top_p));
    }
    if let Some(thinking) = crate::llm::build_thinking_param(&config.model.default_model, config.model.thinking_budget) {
        body["thinking"] = thinking;
    }

    let resp = client.post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", &config.api.api_version)
        .header("content-type", "application/json")
        .json(&body).send().await?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!("Anthropic API error ({}): {}", status, resp.text().await.unwrap_or_default());
    }

    let mut full_text = String::new();
    let mut thinking_text = String::new();
    let mut tool_uses: Vec<crate::llm::ToolCallInfo> = Vec::new();
    let mut stop_reason = "end_turn".to_string();
    let mut usage: Option<crate::llm::UsageInfo> = None;
    let mut current_tool_id = String::new();
    let mut current_tool_name = String::new();
    let mut current_tool_input = String::new();

    let mut stream = resp.bytes_stream();
    use futures_util::StreamExt;
    let mut sse_buffer = String::with_capacity(4096);

    while let Some(chunk) = stream.next().await {
        let chunk = chunk.map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;

        if crate::llm::is_cancelled(conversation_id) {
            log::info!("[Streaming:Anthropic] Cancelled by user during stream, aborting");
            break;
        }

        sse_buffer.push_str(&String::from_utf8_lossy(&chunk));

        while let Some(line_end) = sse_buffer.find('\n') {
            let line = sse_buffer.drain(..line_end).collect::<String>();
            if !sse_buffer.is_empty() { sse_buffer.drain(..1); }

            let line = line.trim();
            if !line.starts_with("data: ") { continue; }
            let data = &line.as_bytes()[6..];

            if let Ok(evt) = serde_json::from_str::<serde_json::Value>(std::str::from_utf8(data).unwrap_or("")) {
                match evt["type"].as_str().unwrap_or("") {
                    "content_block_delta" => {
                        if let Some(delta) = evt.get("delta") {
                            match delta["type"].as_str().unwrap_or("") {
                                "thinking_delta" => {
                                    if let Some(text) = delta["thinking"].as_str() {
                                        thinking_text.push_str(text);
                                        emit_chat_stream(app_handle, serde_json::json!({
                                            "type": "thinking", "conversation_id": conversation_id, "content": text
                                        })).ok();
                                        tokio::task::yield_now().await;
                                    }
                                }
                                "text_delta" => {
                                    if let Some(text) = delta["text"].as_str() {
                                        full_text.push_str(text);
                                        emit_chat_stream(app_handle, serde_json::json!({
                                            "type": "token", "conversation_id": conversation_id, "content": text
                                        })).ok();
                                        tokio::task::yield_now().await;
                                    }
                                }
                                "input_json_delta" => {
                                    if let Some(partial) = delta["partial_json"].as_str() {
                                        current_tool_input.push_str(partial);
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_start" => {
                        if let Some(cb) = evt.get("content_block") {
                            match cb["type"].as_str().unwrap_or("") {
                                "thinking" => {
                                    log::info!("[LLM:Stream] Thinking block started");
                                }
                                "tool_use" => {
                                    current_tool_id = cb["id"].as_str().unwrap_or("").to_string();
                                    current_tool_name = restore_tool_name_from_api(cb["name"].as_str().unwrap_or(""));
                                    current_tool_input.clear();
                                }
                                _ => {}
                            }
                        }
                    }
                    "content_block_stop" => {
                        if !current_tool_id.is_empty() {
                            let input = serde_json::from_str(&current_tool_input).unwrap_or(serde_json::Value::Null);
                            tool_uses.push(crate::llm::ToolCallInfo {
                                id: std::mem::take(&mut current_tool_id),
                                name: std::mem::take(&mut current_tool_name),
                                input,
                            });
                            current_tool_input.clear();
                        }
                    }
                    "message_delta" => {
                        if let Some(delta) = evt.get("delta") {
                            if let Some(sr) = delta["stop_reason"].as_str() { stop_reason = sr.to_string(); }
                        }
                        if let Some(u) = evt.get("usage") {
                            usage = Some(crate::llm::UsageInfo {
                                input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                                output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                                cache_read_tokens: u.get("cache_read_input_tokens").and_then(|v| v.as_u64()),
                                cache_creation_tokens: u.get("cache_creation_input_tokens").and_then(|v| v.as_u64()),
                            });
                        }
                    }
                    "message_start" => {
                        if let Some(msg) = evt.get("message") {
                            if let Some(u) = msg.get("usage") {
                                usage = Some(crate::llm::UsageInfo {
                                    input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                                    output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
                                    cache_read_tokens: u.get("cache_read_input_tokens").and_then(|v| v.as_u64()),
                                    cache_creation_tokens: u.get("cache_creation_input_tokens").and_then(|v| v.as_u64()),
                                });
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let reasoning_out = if thinking_text.is_empty() { None } else { Some(thinking_text) };
    Ok((full_text, tool_uses, stop_reason, usage, reasoning_out))
}
