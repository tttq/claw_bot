// Claw Desktop - 工具循环执行器
// 实现Agent的核心工具调用循环：发送消息→解析工具调用→执行工具→回传结果→继续对话，
// 包含最大轮次限制、同工具连续调用检测、总超时控制、增量保存等安全机制
use super::api_client::{call_anthropic_with_tools, call_openai_with_tools};
use super::connection_health::{
    ConnectionHealthChecker, estimate_tokens_approx, should_compress_preflight,
};
use super::constants::*;
use super::encoding_recovery::{EncodingRecoveryState, sanitize_surrogates_in_string};
use super::error_classifier::LlmErrorType;
use super::loop_detector::{LoopDetector, LoopStatus};
use super::message_sanitizer::{deduplicate_tool_calls, sanitize_messages};
use super::prompt_builder::PromptBuilder;
use super::streaming::{call_anthropic_streaming, call_openai_streaming, emit_chat_stream};
use super::tool_deduplicator::ToolCallDeduplicator;
use claw_config::config::AppConfig;
use claw_types::common::ToolDefinition;

/// API响应内部类型：(响应文本, 工具调用列表, 停止原因, 用量信息, 推理内容)
type ApiResponseInner = (
    String,
    Vec<crate::llm::ToolCallInfo>,
    String,
    Option<crate::llm::UsageInfo>,
    Option<String>,
);

/// 工具循环执行结果
pub struct ToolLoopResult {
    /// 最终汇总的文本内容
    pub final_content: String,
    /// 工具调用总次数
    pub tool_calls_count: usize,
    /// 总执行耗时（毫秒）
    pub total_duration_ms: u64,
}

/// 执行工具循环 — Agent核心执行引擎
///
/// 循环流程：发送消息给LLM → 解析工具调用 → 执行工具 → 回传结果 → 继续对话
/// 安全机制：最大轮次限制、同工具连续调用检测、总超时控制、增量保存、
/// 上下文溢出压缩、编码错误恢复、空响应恢复、死循环检测
pub async fn execute_tool_loop(
    config: &AppConfig,
    conversation_id: &str,
    user_message: &str,
    messages_for_api: &mut Vec<serde_json::Value>,
    tools: &[ToolDefinition],
    app_handle: Option<&tauri::AppHandle>,
    is_streaming: bool,
    agent_max_turns: Option<usize>,
) -> Result<
    (
        String,
        Vec<crate::llm::ToolCallInfo>,
        Vec<crate::llm::ToolExecutionInfo>,
        Option<crate::llm::UsageInfo>,
    ),
    String,
> {
    let mut all_text = String::new();
    let mut all_tool_calls: Vec<crate::llm::ToolCallInfo> = Vec::new();
    let mut all_tool_executions: Vec<crate::llm::ToolExecutionInfo> = Vec::new();
    let mut final_usage: Option<crate::llm::UsageInfo> = None;

    let mut loop_detector = LoopDetector::new();
    let mut tool_deduplicator = ToolCallDeduplicator::new();
    let loop_start_time = std::time::Instant::now();
    let mut api_retry_count: usize = 0;
    let mut overflow_retry_count: usize = 0;
    let mut consecutive_empty_responses: usize = 0;
    let mut total_api_calls: usize = 0;
    let mut compaction_count: usize = 0;
    let mut length_continuation_retries: usize = 0;
    let mut truncated_tool_call_retries: usize = 0;
    let mut compression_attempts: usize = 0;
    let mut truncated_response_prefix: String = String::new();

    let connection_health = ConnectionHealthChecker::new(5);
    let encoding_recovery = EncodingRecoveryState::new(2);

    sanitize_messages(messages_for_api);

    if should_compress_preflight(
        messages_for_api,
        config.advanced.auto_compact_tokens as usize,
        2,
        2,
    ) {
        log::info!(
            "[LLM:Loop] Pre-flight context compression triggered (estimated_tokens={})",
            estimate_tokens_approx(messages_for_api)
        );
        let _ = claw_rag::rag::compact_conversation_if_needed(
            conversation_id,
            None,
            &config.model.default_model,
            Some(config.advanced.auto_compact_tokens / 2),
        )
        .await;
    }

    let actual_max_rounds = agent_max_turns
        .unwrap_or(MAX_TOOL_ROUNDS)
        .min(MAX_TOOL_ROUNDS);

    for round in 1..=actual_max_rounds {
        if crate::llm::is_cancelled(conversation_id) {
            log::info!("[LLM:Loop] Cancelled by user at round {}, stopping", round);
            all_text.push_str(&format!(
                "\n\n[System Notice]: Agent loop cancelled by user at round {}.",
                round
            ));
            break;
        }

        if loop_start_time.elapsed().as_secs() >= TOTAL_LOOP_TIMEOUT_SECS {
            log::warn!(
                "[LLM:Loop] TIMEOUT: total loop exceeded {}s at round {}, forcing break",
                TOTAL_LOOP_TIMEOUT_SECS,
                round
            );
            all_text.push_str(&format!("\n\n[System Notice]: Agent loop exceeded {} second timeout (round {}). Stopping tool execution and generating final response based on gathered information.", TOTAL_LOOP_TIMEOUT_SECS, round));
            break;
        }

        log::info!(
            "[LLM:Loop] === Round {}/{} | status={:?} | api_retries={} | overflow_retries={} | continuations={} | compressions={} | conn_failures={} ===",
            round,
            actual_max_rounds,
            loop_detector.check(),
            api_retry_count,
            overflow_retry_count,
            length_continuation_retries,
            compression_attempts,
            connection_health.get_consecutive_failures()
        );

        sanitize_messages(messages_for_api);

        tool_deduplicator.reset();

        if connection_health.should_trigger_recovery() {
            log::warn!(
                "[LLM:Loop] Connection health threshold exceeded ({} failures), attempting recovery",
                connection_health.get_consecutive_failures()
            );
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            connection_health.reset();
        }

        let result_inner: ApiResponseInner = if is_streaming {
            let handle = match app_handle {
                Some(h) => h,
                None => continue,
            };
            execute_streaming_api_call(
                config,
                conversation_id,
                messages_for_api,
                tools,
                handle,
                &mut api_retry_count,
                &mut overflow_retry_count,
                &mut total_api_calls,
                &mut compaction_count,
                &mut compression_attempts,
                round,
                &mut all_text,
                &mut all_tool_calls,
                &mut all_tool_executions,
                &mut final_usage,
                &mut length_continuation_retries,
                &mut truncated_tool_call_retries,
                &mut truncated_response_prefix,
                &connection_health,
                &encoding_recovery,
            )
            .await?
        } else {
            execute_non_streaming_api_call(
                config,
                conversation_id,
                messages_for_api,
                tools,
                &mut api_retry_count,
                &mut overflow_retry_count,
                &mut total_api_calls,
                &mut compaction_count,
                &mut compression_attempts,
                round,
                &mut all_text,
                &mut all_tool_calls,
                &mut all_tool_executions,
                &mut final_usage,
                &mut length_continuation_retries,
                &mut truncated_tool_call_retries,
                &mut truncated_response_prefix,
                &connection_health,
                &encoding_recovery,
            )
            .await?
        };

        connection_health.record_success();
        encoding_recovery.reset();

        let response_text = result_inner.0;
        let tool_uses = result_inner.1;
        let stop_reason = result_inner.2;
        let usage = result_inner.3;
        let reasoning_content = result_inner.4;

        let elapsed = loop_start_time.elapsed().as_millis();
        log::info!(
            "[LLM:Loop] Round {} done in {}ms | text={} | tools={} | stop={}",
            round,
            elapsed,
            response_text.len(),
            tool_uses.len(),
            stop_reason
        );

        if !response_text.is_empty() {
            all_text.push_str(&response_text);
            if !all_text.ends_with('\n') {
                all_text.push('\n');
            }
            consecutive_empty_responses = 0;
            length_continuation_retries = 0;
            truncated_response_prefix.clear();
        } else if tool_uses.is_empty() {
            consecutive_empty_responses += 1;
            log::warn!(
                "[LLM:Loop] Empty response from LLM (round {}, consecutive={})",
                round,
                consecutive_empty_responses
            );
            if consecutive_empty_responses >= 2 {
                log::warn!(
                    "[LLM:Loop] Multiple empty responses detected, injecting recovery prompt"
                );
                messages_for_api.push(serde_json::json!({
                    "role": "user",
                    "content": "[System Notice]: Your previous responses were empty. Please provide a substantive answer or explanation based on the available information and tool results."
                }));
                consecutive_empty_responses = 0;
                continue;
            }
        } else {
            consecutive_empty_responses = 0;
        }
        all_tool_calls.extend(tool_uses.clone());
        final_usage = usage;

        if round % INCREMENTAL_SAVE_INTERVAL == 0 && !all_text.is_empty() {
            if let Err(e) =
                crate::llm::store_interaction_to_rag(conversation_id, None, user_message, &all_text)
                    .await
            {
                log::warn!(
                    "[LLM:Loop] Incremental memory save failed (round {}): {}",
                    round,
                    e
                );
            }
        }

        if tool_uses.is_empty() || stop_reason == "end_turn" || stop_reason == "stop" {
            break;
        }

        if stop_reason == "max_tokens" || stop_reason == "length" {
            log::warn!(
                "[LLM:Loop] Round {} STOP: output truncated (stop_reason={}). Response may be incomplete.",
                round,
                stop_reason
            );
            break;
        }

        if config.is_openai_compatible() {
            let mut tc_array = Vec::new();
            for tc in &tool_uses {
                tc_array.push(serde_json::json!({
                    "id": tc.id, "type": "function",
                    "function": { "name": crate::streaming::sanitize_tool_name_for_api(&tc.name), "arguments": serde_json::to_string(&tc.input).unwrap_or_default() }
                }));
            }

            let deduplicated_tc_array = deduplicate_tool_calls(&tc_array);

            let mut assistant_msg = serde_json::json!({
                "role": "assistant",
                "content": if response_text.is_empty() { serde_json::Value::Null } else { serde_json::json!(response_text) },
                "tool_calls": deduplicated_tc_array
            });
            if let Some(ref rc) = reasoning_content {
                assistant_msg["reasoning_content"] = serde_json::json!(rc);
            }
            messages_for_api.push(assistant_msg);
        } else {
            let assistant_content = PromptBuilder::build_assistant_content_from_tool_uses(
                &tool_uses,
                &response_text,
                reasoning_content.as_deref(),
            );
            messages_for_api
                .push(serde_json::json!({"role": "assistant", "content": assistant_content}));
        }

        let mut should_break_loop = false;

        for tc in &tool_uses {
            let stream_handle = if is_streaming { app_handle } else { None };

            if let Some(h) = stream_handle {
                let input_preview: String = serde_json::to_string(&tc.input)
                    .unwrap_or_default()
                    .chars()
                    .take(500)
                    .collect();
                emit_chat_stream(
                    h,
                    serde_json::json!({
                        "type": "tool_execution",
                        "conversation_id": conversation_id,
                        "tool_name": tc.name,
                        "tool_index": all_tool_executions.len() + 1,
                        "round": round,
                        "tool_input": input_preview,
                        "status": "running"
                    }),
                )
                .ok();
            }

            let args_str = serde_json::to_string(&tc.input).unwrap_or_default();
            if tool_deduplicator.is_duplicate(&tc.name, &args_str) {
                log::warn!(
                    "[LLM:Loop] Skipping duplicate tool call: {} (round {})",
                    tc.name,
                    round
                );

                if config.is_openai_compatible() {
                    messages_for_api.push(serde_json::json!({
                        "role": "tool",
                        "tool_call_id": tc.id,
                        "content": "[Duplicate call skipped - already executed in this turn]"
                    }));
                } else {
                    messages_for_api.push(serde_json::json!({"role": "user", "content": [
                        {"type": "tool_result", "tool_use_id": tc.id, "content": "[Duplicate call skipped]"}
                    ]}));
                }
                continue;
            }

            let exec_start = std::time::Instant::now();

            if crate::llm::is_cancelled(conversation_id) {
                log::info!(
                    "[LLM:Loop] Cancelled by user before tool execution '{}', stopping",
                    tc.name
                );
                all_text.push_str(&format!("\n\n[System Notice]: Agent loop cancelled by user. Tool '{}' was not executed.", tc.name));
                break;
            }

            let raw_tool_result: String = crate::llm::execute_tool(&tc.name, &tc.input).await;
            let raw_len = raw_tool_result.len();
            let exec_ms = exec_start.elapsed().as_millis();

            let (tool_result, was_truncated) = if raw_tool_result.len() > 4096 {
                let mut end = 4096;
                while end > 0 && !raw_tool_result.is_char_boundary(end) {
                    end -= 1;
                }
                let truncated = format!(
                    "{}...\n\n[Output truncated: {} chars total, showing first 4096. Full output stored in RAG memory.]",
                    &raw_tool_result[..end],
                    raw_tool_result.len()
                );
                if conversation_id.len() > 0 && !raw_tool_result.trim().is_empty() {
                    claw_rag::rag::store_enhanced_memory(
                        "default",
                        Some(conversation_id),
                        &format!("[Tool:{}]\n{}", tc.name, raw_tool_result),
                        "observation",
                        "tool_output",
                        None,
                        None,
                    )
                    .await
                    .map_err(|e| {
                        log::warn!(
                            "[ToolLoop:execute_tool] store_enhanced_memory failed: {}",
                            e
                        );
                        e
                    })
                    .ok();
                }
                (truncated, true)
            } else {
                (raw_tool_result, false)
            };

            all_tool_executions.push(crate::llm::ToolExecutionInfo {
                round,
                tool_name: tc.name.clone(),
                tool_input: tc.input.clone(),
                tool_result: if was_truncated {
                    format!("[TRUNCATED {}→{} chars]", raw_len, tool_result.len())
                } else {
                    tool_result.clone()
                },
                duration_ms: exec_ms,
            });

            let result_preview: String = tool_result.chars().take(300).collect();

            if let Some(h) = stream_handle {
                let result_preview_for_event: String = tool_result.chars().take(1000).collect();
                emit_chat_stream(h, serde_json::json!({
                    "type": "tool_execution",
                    "conversation_id": conversation_id,
                    "tool_name": tc.name,
                    "duration_ms": exec_ms,
                    "tool_index": all_tool_executions.len(),
                    "total_tools": all_tool_executions.len(),
                    "round": round,
                    "tool_input": serde_json::to_string(&tc.input).unwrap_or_default().chars().take(500).collect::<String>(),
                    "tool_result": result_preview_for_event,
                    "status": "completed"
                })).ok();
            }

            match loop_detector.record(&tc.name, &args_str, &result_preview) {
                LoopStatus::Normal => {}
                LoopStatus::Warning(_) => {
                    all_text.push_str(&format!("\n[System Warning]: Repeated or non-productive tool calls detected (round {}). Continuing cautiously...\n", round));
                    if let Some(h) = stream_handle {
                        emit_chat_stream(h, serde_json::json!({"type": "token", "conversation_id": conversation_id, "content": format!("\n[System Warning]: Repeated or non-productive tool calls detected (round {}). Continuing cautiously...\n", round)})).ok();
                    }
                }
                LoopStatus::Blocked(_) => {
                    all_text.push_str(&format!("\n[System Notice]: Loop detected (round {}). Stopping.\nPlease synthesize your response from available information.\n", round));
                    if let Some(h) = stream_handle {
                        emit_chat_stream(h, serde_json::json!({"type": "token", "conversation_id": conversation_id, "content": format!("\n[System Notice]: Loop detected (round {}). Stopping.\nPlease synthesize your response from available information.\n", round)})).ok();
                    }
                    should_break_loop = true;
                    break;
                }
                LoopStatus::Broken(_) => {
                    all_text.push_str(&format!(
                        "\n[System Notice]: Infinite loop (round {}). Stopping immediately.\n",
                        round
                    ));
                    if let Some(h) = stream_handle {
                        emit_chat_stream(h, serde_json::json!({"type": "token", "conversation_id": conversation_id, "content": format!("\n[System Notice]: Infinite loop detected (round {}). Stopping immediately.\n", round)})).ok();
                    }
                    should_break_loop = true;
                    break;
                }
            }

            if config.is_openai_compatible() {
                messages_for_api.push(serde_json::json!({ "role": "tool", "tool_call_id": tc.id, "content": tool_result }));
            } else {
                messages_for_api.push(serde_json::json!({"role": "user", "content": [
                    {"type": "tool_result", "tool_use_id": tc.id, "content": tool_result}
                ]}));
            }
        }
        if should_break_loop {
            break;
        }
    }

    Ok((all_text, all_tool_calls, all_tool_executions, final_usage))
}

/// 非流式API调用执行器
///
/// 处理非流式模式下的LLM API调用，包含完整的错误恢复策略：
/// - 上下文溢出 → RAG压缩重试
/// - 输出截断 → 长度续写重试
/// - 工具调用截断 → 重试不完整参数
/// - 编码错误 → Unicode清理重试
/// - Thinking签名错误 → 剥离thinking块重试
/// - 限流/服务器/网络错误 → 指数退避重试
async fn execute_non_streaming_api_call(
    config: &AppConfig,
    conversation_id: &str,
    messages_for_api: &mut Vec<serde_json::Value>,
    tools: &[ToolDefinition],
    api_retry_count: &mut usize,
    overflow_retry_count: &mut usize,
    total_api_calls: &mut usize,
    compaction_count: &mut usize,
    compression_attempts: &mut usize,
    round: usize,
    all_text: &mut String,
    _all_tool_calls: &mut Vec<crate::llm::ToolCallInfo>,
    _all_tool_executions: &mut Vec<crate::llm::ToolExecutionInfo>,
    _final_usage: &mut Option<crate::llm::UsageInfo>,
    length_continuation_retries: &mut usize,
    truncated_tool_call_retries: &mut usize,
    truncated_response_prefix: &mut String,
    connection_health: &ConnectionHealthChecker,
    encoding_recovery: &EncodingRecoveryState,
) -> Result<ApiResponseInner, String> {
    const MAX_LENGTH_CONTINUATION_RETRIES: usize = 3;
    const MAX_COMPRESSION_ATTEMPTS: usize = 3;

    let mut last_error: Option<(LlmErrorType, String)> = None;
    let mut attempt_result: Option<ApiResponseInner> = None;
    let mut should_retry_with_compression = false;
    let mut should_retry_length_continuation = false;
    let mut should_retry_truncated_tool_call = false;
    let mut should_retry_encoding_error = false;

    for retry in 0..=MAX_API_RETRIES {
        if should_retry_with_compression {
            should_retry_with_compression = false;
            *compression_attempts += 1;

            if *compression_attempts > MAX_COMPRESSION_ATTEMPTS {
                log::error!(
                    "[LLM:Loop] Max compression attempts ({}) reached",
                    MAX_COMPRESSION_ATTEMPTS
                );
                return Err(format!(
                    "Request payload too large: max compression attempts ({}) reached.",
                    MAX_COMPRESSION_ATTEMPTS
                ));
            }

            let original_len = messages_for_api.len();
            if let Err(compact_err) = claw_rag::rag::compact_conversation_if_needed(
                conversation_id,
                None,
                &config.model.default_model,
                Some(config.advanced.auto_compact_tokens / 2),
            )
            .await
            {
                log::warn!("[LLM:Loop] Compaction failed: {}", compact_err);
            }

            if messages_for_api.len() < original_len {
                log::info!(
                    "[LLM:Loop] Compressed {} → {} messages, retrying...",
                    original_len,
                    messages_for_api.len()
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            } else {
                return Err("Request payload too large. Cannot compress further.".to_string());
            }
        }

        if should_retry_length_continuation {
            should_retry_length_continuation = false;
            *length_continuation_retries += 1;

            if *length_continuation_retries < MAX_LENGTH_CONTINUATION_RETRIES {
                log::info!(
                    "[LLM:Loop] Requesting length continuation ({}/{})",
                    *length_continuation_retries,
                    MAX_LENGTH_CONTINUATION_RETRIES
                );

                messages_for_api.push(serde_json::json!({
                    "role": "user",
                    "content": "[System: Your previous response was truncated by the output length limit. Continue exactly where you left off. Do not restart or repeat prior text. Finish the answer directly.]"
                }));
                continue;
            } else {
                let _partial_response = truncated_response_prefix.clone();
                return Err(format!(
                    "Response remained truncated after {} continuation attempts",
                    MAX_LENGTH_CONTINUATION_RETRIES
                ));
            }
        }

        if should_retry_truncated_tool_call {
            should_retry_truncated_tool_call = false;
            *truncated_tool_call_retries += 1;

            if *truncated_tool_call_retries <= 2 {
                log::info!(
                    "[LLM:Loop] Retrying truncated tool call response ({}/2)",
                    *truncated_tool_call_retries
                );
                continue;
            } else {
                return Err(
                    "Response truncated due to incomplete tool call arguments after 2 retries"
                        .to_string(),
                );
            }
        }

        if should_retry_encoding_error {
            should_retry_encoding_error = false;

            if encoding_recovery.should_attempt_sanitization() {
                encoding_recovery.record_sanitization_pass();
                let sanitization_passes = encoding_recovery
                    .unicode_sanitization_passes
                    .lock()
                    .map_err(|e| format!("[LLM:Loop] Failed to acquire lock: {}", e))?;
                log::info!(
                    "[LLM:Loop] Sanitizing encoding errors (pass {}/{})",
                    sanitization_passes,
                    encoding_recovery.max_sanitization_passes
                );

                for msg in messages_for_api.iter_mut() {
                    if let Some(content) = msg.get_mut("content") {
                        if let Some(s) = content.as_str() {
                            let sanitized = sanitize_surrogates_in_string(s);
                            *content = serde_json::Value::String(sanitized);
                        }
                    }
                }
                continue;
            } else {
                return Err("Encoding error recovery exhausted.".to_string());
            }
        }

        *total_api_calls += 1;
        let call_result: std::result::Result<ApiResponseInner, anyhow::Error> =
            if config.is_openai_compatible() {
                call_openai_with_tools(
                    crate::llm::http_client(),
                    &config.get_base_url(),
                    &config.resolve_api_key().map_err(|e| e.to_string())?,
                    config,
                    messages_for_api,
                    tools,
                )
                .await
            } else {
                call_anthropic_with_tools(
                    crate::llm::http_client(),
                    &config.get_base_url(),
                    &config.resolve_api_key().map_err(|e| e.to_string())?,
                    config,
                    messages_for_api,
                    tools,
                )
                .await
            };

        match call_result {
            Ok(result) => {
                let stop_reason = &result.2;

                if stop_reason == "length" || stop_reason == "max_tokens" {
                    let response_text = &result.0;
                    let tool_uses = &result.1;

                    if !tool_uses.is_empty() {
                        if *truncated_tool_call_retries < 2 {
                            log::warn!(
                                "[LLM:Loop] Truncated tool call detected - retrying API call"
                            );
                            should_retry_truncated_tool_call = true;
                            continue;
                        } else {
                            log::warn!(
                                "[LLM:Loop] Truncated tool call retry exhausted - refusing incomplete arguments"
                            );
                            return Err("Response truncated due to incomplete tool call arguments after 2 retries".to_string());
                        }
                    }

                    if !response_text.is_empty() {
                        truncated_response_prefix.push_str(response_text);
                        should_retry_length_continuation = true;
                        continue;
                    }
                }

                attempt_result = Some(result);
                *api_retry_count = 0;
                break;
            }
            Err(e) => {
                connection_health.record_failure();
                let error_str: String = e.to_string();
                let error_type = classify_llm_error(&error_str, None);
                last_error = Some((error_type.clone(), error_str.clone()));

                let mut err_end = std::cmp::min(200, error_str.len());
                while err_end > 0 && !error_str.is_char_boundary(err_end) {
                    err_end -= 1;
                }
                log::warn!(
                    "[LLM:Loop] API error on attempt {}/{} (round {}): type={:?} | {}",
                    retry + 1,
                    MAX_API_RETRIES + 1,
                    round,
                    error_type,
                    &error_str[..err_end]
                );

                let msg_lower = error_str.to_lowercase();
                let is_payload_too_large = msg_lower.contains("413")
                    || msg_lower.contains("payload too large")
                    || msg_lower.contains("request entity too large");

                if is_payload_too_large {
                    should_retry_with_compression = true;
                    continue;
                }

                let is_encoding_error = msg_lower.contains("surrogate")
                    || msg_lower.contains("encode")
                    || msg_lower.contains("ascii");

                if is_encoding_error {
                    should_retry_encoding_error = true;
                    continue;
                }

                let is_thinking_signature = msg_lower.contains("thinking")
                    && (msg_lower.contains("signature")
                        || msg_lower.contains("tampered")
                        || msg_lower.contains("invalid"));

                if is_thinking_signature {
                    log::warn!(
                        "[LLM:Loop] ThinkingSignature error detected, stripping thinking blocks and retrying"
                    );
                    for msg in messages_for_api.iter_mut() {
                        if msg.get("role").and_then(|v| v.as_str()) == Some("assistant") {
                            if let Some(obj) = msg.as_object_mut() {
                                obj.remove("reasoning_content");
                                obj.remove("reasoning");
                                if let Some(content) = obj.get_mut("content") {
                                    if let Some(arr) = content.as_array_mut() {
                                        arr.retain(|part| {
                                            part.get("type").and_then(|v| v.as_str())
                                                != Some("thinking")
                                        });
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                match &error_type {
                    LlmErrorType::ContextOverflow => {
                        if *overflow_retry_count < CONTEXT_OVERFLOW_MAX_RETRIES {
                            log::warn!(
                                "[LLM:Loop] Context overflow detected ({}/{}), attempting compaction...",
                                *overflow_retry_count + 1,
                                CONTEXT_OVERFLOW_MAX_RETRIES
                            );
                            if let Err(compact_err) = claw_rag::rag::compact_conversation_if_needed(
                                conversation_id,
                                None,
                                &config.model.default_model,
                                Some(config.advanced.auto_compact_tokens / 2),
                            )
                            .await
                            {
                                log::warn!("[LLM:Loop] Compaction failed: {}", compact_err);
                            }
                            *overflow_retry_count += 1;
                            *compaction_count += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            continue;
                        }
                        log::error!("[LLM:Loop] Context overflow retries exhausted");
                        let user_msg = format_error_for_user(&error_type, &error_str);
                        all_text.push_str(&format!("\n\n[Error]: {}\n", user_msg));
                        return Err(format!("{}: {}", user_msg, error_str));
                    }
                    LlmErrorType::RateLimit
                    | LlmErrorType::ServerError
                    | LlmErrorType::NetworkError
                    | LlmErrorType::Timeout => {
                        if should_retry_error(&error_type, retry) {
                            let delay_ms = get_retry_delay_ms(&error_type, retry);
                            log::info!(
                                "[LLM:Loop] Retrying in {}ms (attempt {}/{})",
                                delay_ms,
                                retry + 1,
                                MAX_API_RETRIES + 1
                            );
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            *api_retry_count = retry + 1;
                            continue;
                        }
                    }
                    _ => {}
                }

                if retry == MAX_API_RETRIES || !should_retry_error(&error_type, retry) {
                    let user_msg = format_error_for_user(&error_type, &error_str);
                    log::error!(
                        "[LLM:Loop] All retries exhausted or non-retryable error: {:?}",
                        error_type
                    );
                    all_text.push_str(&format!("\n\n[Error]: {}\n", user_msg));

                    crate::llm::store_interaction_to_rag(conversation_id, None, "", &all_text)
                        .await
                        .map_err(|e| {
                            log::warn!(
                                "[ToolLoop:run] store_interaction_to_rag failed on error: {}",
                                e
                            );
                            e
                        })
                        .ok();
                    return Err(format!("{}: {}", user_msg, error_str));
                }
            }
        }
    }

    match attempt_result {
        Some(result) => Ok(result),
        None => {
            let (err_type, err_str) =
                last_error.unwrap_or((LlmErrorType::Unknown, "Unknown error".to_string()));
            Err(format!(
                "{}: {}",
                format_error_for_user(&err_type, &err_str),
                err_str
            ))
        }
    }
}

/// 流式API调用执行器
///
/// 处理流式模式下的LLM API调用，带120秒超时控制，
/// 包含与非流式相同的错误恢复策略，额外支持流式错误事件推送
async fn execute_streaming_api_call(
    config: &AppConfig,
    conversation_id: &str,
    messages_for_api: &mut Vec<serde_json::Value>,
    tools: &[ToolDefinition],
    app_handle: &tauri::AppHandle,
    _stream_api_retries: &mut usize,
    stream_overflow_retries: &mut usize,
    _stream_total_api_calls: &mut usize,
    _stream_compaction_count: &mut usize,
    compression_attempts: &mut usize,
    round: usize,
    _all_text: &mut String,
    _all_tool_calls: &mut Vec<crate::llm::ToolCallInfo>,
    _all_tool_executions: &mut Vec<crate::llm::ToolExecutionInfo>,
    _final_usage: &mut Option<crate::llm::UsageInfo>,
    length_continuation_retries: &mut usize,
    truncated_tool_call_retries: &mut usize,
    truncated_response_prefix: &mut String,
    connection_health: &ConnectionHealthChecker,
    encoding_recovery: &EncodingRecoveryState,
) -> Result<ApiResponseInner, String> {
    const LLM_CALL_TIMEOUT_SECS: u64 = 120;
    const MAX_LENGTH_CONTINUATION_RETRIES: usize = 3;
    const MAX_COMPRESSION_ATTEMPTS: usize = 3;

    let mut last_stream_error: Option<(LlmErrorType, String)> = None;
    let mut stream_attempt_result: Option<ApiResponseInner> = None;
    let mut should_retry_with_compression = false;
    let mut should_retry_length_continuation = false;
    let mut should_retry_truncated_tool_call = false;
    let mut should_retry_encoding_error = false;

    let base_url = config.get_base_url();
    let api_key = config.resolve_api_key().map_err(|e| e.to_string())?;

    for retry in 0..=MAX_API_RETRIES {
        if should_retry_with_compression {
            should_retry_with_compression = false;
            *compression_attempts += 1;

            if *compression_attempts > MAX_COMPRESSION_ATTEMPTS {
                let user_msg = format!(
                    "Request payload too large: max compression attempts ({}) reached.",
                    MAX_COMPRESSION_ATTEMPTS
                );
                let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                return Err(user_msg);
            }

            let original_len = messages_for_api.len();
            let _ = claw_rag::rag::compact_conversation_if_needed(
                conversation_id,
                None,
                &config.model.default_model,
                Some(config.advanced.auto_compact_tokens / 2),
            )
            .await;

            if messages_for_api.len() < original_len {
                log::info!(
                    "[LLM:Stream] Compressed {} → {} messages, retrying...",
                    original_len,
                    messages_for_api.len()
                );
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                continue;
            } else {
                let user_msg = "Request payload too large. Cannot compress further.".to_string();
                let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                return Err(user_msg);
            }
        }

        if should_retry_length_continuation {
            should_retry_length_continuation = false;
            *length_continuation_retries += 1;

            if *length_continuation_retries < MAX_LENGTH_CONTINUATION_RETRIES {
                log::info!(
                    "[LLM:Stream] Requesting length continuation ({}/{})",
                    *length_continuation_retries,
                    MAX_LENGTH_CONTINUATION_RETRIES
                );

                messages_for_api.push(serde_json::json!({
                    "role": "user",
                    "content": "[System: Your previous response was truncated by the output length limit. Continue exactly where you left off. Do not restart or repeat prior text. Finish the answer directly.]"
                }));
                continue;
            } else {
                let user_msg = format!(
                    "Response remained truncated after {} continuation attempts",
                    MAX_LENGTH_CONTINUATION_RETRIES
                );
                let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                return Err(user_msg);
            }
        }

        if should_retry_truncated_tool_call {
            should_retry_truncated_tool_call = false;
            *truncated_tool_call_retries += 1;

            if *truncated_tool_call_retries <= 2 {
                log::info!(
                    "[LLM:Stream] Retrying truncated tool call response ({}/2)",
                    *truncated_tool_call_retries
                );
                continue;
            } else {
                let user_msg =
                    "Response truncated due to incomplete tool call arguments after 2 retries"
                        .to_string();
                let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                return Err(user_msg);
            }
        }

        if should_retry_encoding_error {
            should_retry_encoding_error = false;

            if encoding_recovery.should_attempt_sanitization() {
                encoding_recovery.record_sanitization_pass();
                let sanitization_passes = encoding_recovery
                    .unicode_sanitization_passes
                    .lock()
                    .map_err(|e| format!("[LLM:Stream] Failed to acquire lock: {}", e))?;
                log::info!(
                    "[LLM:Stream] Sanitizing encoding errors (pass {}/{})",
                    sanitization_passes,
                    encoding_recovery.max_sanitization_passes
                );

                for msg in messages_for_api.iter_mut() {
                    if let Some(content) = msg.get_mut("content") {
                        if let Some(s) = content.as_str() {
                            let sanitized = sanitize_surrogates_in_string(s);
                            *content = serde_json::Value::String(sanitized);
                        }
                    }
                }
                continue;
            } else {
                let user_msg = "Encoding error recovery exhausted.".to_string();
                let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                return Err(user_msg);
            }
        }

        let use_openai = config.is_openai_compatible();
        let proto_name = if use_openai {
            "OpenAI(Bearer)"
        } else {
            "Anthropic(x-api-key)"
        };
        log::info!(
            "[LLM:Stream:Diag] protocol={} api_format={} provider={} base_url={} model={}",
            proto_name,
            config.model.api_format,
            config.model.provider,
            base_url,
            config.model.default_model
        );

        let api_key_clone = api_key.clone();
        let base_url_owned: String = base_url.to_string();

        let timeout_result = if use_openai {
            tokio::time::timeout(
                std::time::Duration::from_secs(LLM_CALL_TIMEOUT_SECS),
                call_openai_streaming(
                    crate::llm::http_client(),
                    &base_url_owned,
                    &api_key_clone,
                    config,
                    messages_for_api,
                    tools,
                    &app_handle,
                    conversation_id,
                ),
            )
            .await
        } else {
            tokio::time::timeout(
                std::time::Duration::from_secs(LLM_CALL_TIMEOUT_SECS),
                call_anthropic_streaming(
                    crate::llm::http_client(),
                    &base_url_owned,
                    &api_key_clone,
                    config,
                    messages_for_api,
                    tools,
                    &app_handle,
                    conversation_id,
                ),
            )
            .await
        };

        match timeout_result {
            Ok(Ok(result)) => {
                let stop_reason = &result.2;

                if stop_reason == "length" || stop_reason == "max_tokens" {
                    let response_text = &result.0;
                    let tool_uses = &result.1;

                    if !tool_uses.is_empty() {
                        if *truncated_tool_call_retries < 2 {
                            log::warn!(
                                "[LLM:Stream] Truncated tool call detected - retrying API call"
                            );
                            should_retry_truncated_tool_call = true;
                            continue;
                        } else {
                            let user_msg = "Response truncated due to incomplete tool call arguments after 2 retries".to_string();
                            let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg.clone()})).ok();
                            return Err(user_msg);
                        }
                    }

                    if !response_text.is_empty() {
                        truncated_response_prefix.push_str(response_text);
                        should_retry_length_continuation = true;
                        continue;
                    }
                }

                stream_attempt_result = Some(result);
                break;
            }
            Ok(Err(e)) => {
                connection_health.record_failure();
                let error_str: String = e.to_string();
                let error_type = classify_llm_error(&error_str, None);
                last_stream_error = Some((error_type.clone(), error_str.clone()));

                log::warn!(
                    "[LLM:Stream] API error on attempt {}/{} (round {}): type={:?}",
                    retry + 1,
                    MAX_API_RETRIES + 1,
                    round,
                    error_type
                );

                let msg_lower = error_str.to_lowercase();
                let is_payload_too_large = msg_lower.contains("413")
                    || msg_lower.contains("payload too large")
                    || msg_lower.contains("request entity too large");

                if is_payload_too_large {
                    should_retry_with_compression = true;
                    continue;
                }

                let is_encoding_error = msg_lower.contains("surrogate")
                    || msg_lower.contains("encode")
                    || msg_lower.contains("ascii");

                if is_encoding_error {
                    should_retry_encoding_error = true;
                    continue;
                }

                let is_thinking_signature = msg_lower.contains("thinking")
                    && (msg_lower.contains("signature")
                        || msg_lower.contains("tampered")
                        || msg_lower.contains("invalid"));

                if is_thinking_signature {
                    log::warn!(
                        "[LLM:Stream] ThinkingSignature error detected, stripping thinking blocks and retrying"
                    );
                    for msg in messages_for_api.iter_mut() {
                        if msg.get("role").and_then(|v| v.as_str()) == Some("assistant") {
                            if let Some(obj) = msg.as_object_mut() {
                                obj.remove("reasoning_content");
                                obj.remove("reasoning");
                                if let Some(content) = obj.get_mut("content") {
                                    if let Some(arr) = content.as_array_mut() {
                                        arr.retain(|part| {
                                            part.get("type").and_then(|v| v.as_str())
                                                != Some("thinking")
                                        });
                                    }
                                }
                            }
                        }
                    }
                    continue;
                }

                match &error_type {
                    LlmErrorType::ContextOverflow => {
                        if *stream_overflow_retries < CONTEXT_OVERFLOW_MAX_RETRIES {
                            log::warn!(
                                "[LLM:Stream] Context overflow in streaming mode, attempting compaction..."
                            );
                            let _ = claw_rag::rag::compact_conversation_if_needed(
                                conversation_id,
                                None,
                                &config.model.default_model,
                                Some(config.advanced.auto_compact_tokens / 2),
                            )
                            .await;
                            *stream_overflow_retries += 1;
                            tokio::time::sleep(std::time::Duration::from_millis(500)).await;
                            continue;
                        }
                        let user_msg = format_error_for_user(&error_type, &error_str);
                        let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                        return Err(format!("{}: {}", user_msg, error_str));
                    }
                    LlmErrorType::RateLimit
                    | LlmErrorType::ServerError
                    | LlmErrorType::NetworkError
                    | LlmErrorType::Timeout => {
                        if should_retry_error(&error_type, retry) {
                            let delay_ms = get_retry_delay_ms(&error_type, retry);
                            log::info!("[LLM:Stream] Retrying in {}ms", delay_ms);
                            tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                            continue;
                        }
                    }
                    _ => {}
                }

                if retry == MAX_API_RETRIES || !should_retry_error(&error_type, retry) {
                    let user_msg = format_error_for_user(&error_type, &error_str);
                    let _ = emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": user_msg})).ok();
                    return Err(format!("{}: {}", user_msg, error_str));
                }
            }
            Err(_) => {
                connection_health.record_failure();
                let error_type = LlmErrorType::Timeout;
                log::error!(
                    "[LLM:Stream] LLM call timeout after {}s (round {})",
                    LLM_CALL_TIMEOUT_SECS,
                    round
                );

                if should_retry_error(&error_type, retry) {
                    let delay_ms = get_retry_delay_ms(&error_type, retry);
                    log::info!(
                        "[LLM:Stream] Retrying timeout in {}ms (attempt {}/{})",
                        delay_ms,
                        retry + 1,
                        MAX_API_RETRIES + 1
                    );
                    let _ = emit_chat_stream(&app_handle, serde_json::json!({
                        "type": "token",
                        "conversation_id": conversation_id,
                        "content": format!("\n[System Notice]: LLM call timed out ({}s), retrying in {}ms...\n", LLM_CALL_TIMEOUT_SECS, delay_ms)
                    })).ok();
                    tokio::time::sleep(std::time::Duration::from_millis(delay_ms)).await;
                    continue;
                }

                let user_msg = format!(
                    "LLM API timeout after {} seconds on round {}",
                    LLM_CALL_TIMEOUT_SECS, round
                );
                let _ = emit_chat_stream(
                    &app_handle,
                    serde_json::json!({
                        "type": "error",
                        "conversation_id": conversation_id,
                        "content": user_msg
                    }),
                );
                return Err(user_msg);
            }
        }
    }

    match stream_attempt_result {
        Some(result) => Ok(result),
        None => {
            let (err_type, err_str) = last_stream_error
                .unwrap_or((LlmErrorType::Unknown, "Unknown streaming error".to_string()));
            Err(format!(
                "{}: {}",
                format_error_for_user(&err_type, &err_str),
                err_str
            ))
        }
    }
}
