// Claw Desktop - API客户端 - HTTP流式请求、SSE解析、重试策略
use anyhow::Result;
use claw_config::config::AppConfig;
use claw_types::common::ToolDefinition;
use reqwest::Client;

type ApiResponseInner = (
    String,
    Vec<crate::llm::ToolCallInfo>,
    String,
    Option<crate::llm::UsageInfo>,
    Option<String>,
);

pub(crate) use crate::llm::build_api_url;

/// 调用OpenAI兼容API进行非流式请求，支持工具调用和思考参数
pub async fn call_openai_with_tools(
    client: &Client,
    base_url: &str,
    api_key: &str,
    config: &AppConfig,
    messages: &[serde_json::Value],
    tools: &[ToolDefinition],
) -> Result<ApiResponseInner> {
    let base = base_url.trim_end_matches('/');
    let url = build_api_url(base, "/chat/completions");
    let key_preview = if api_key.len() > 8 {
        let prefix: String = api_key.chars().take(4).collect();
        let suffix: String = api_key
            .chars()
            .skip(api_key.len().saturating_sub(4))
            .collect();
        format!("{}...{} (len={})", prefix, suffix, api_key.len())
    } else {
        format!("(len={})", api_key.len())
    };
    log::info!(
        "[LLM:OpenAI] POST {} key={} model={}",
        url,
        key_preview,
        config.model.default_model
    );

    let api_tools: Vec<serde_json::Value> = tools.iter().map(|t| serde_json::json!({
        "type": "function",
        "function": { "name": crate::streaming::sanitize_tool_name_for_api(&t.name), "description": t.description, "parameters": t.input_schema }
    })).collect();

    let mut body = serde_json::json!({
        "model": config.model.default_model,
        "max_tokens": config.model.max_tokens,
        "stream": false,
        "messages": messages,
        "tools": api_tools
    });

    if !crate::llm::model_ignores_temperature(&config.model.default_model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(
            &config.model.default_model,
            config.model.temperature
        ));
    }
    if !crate::llm::model_ignores_top_p(&config.model.default_model) {
        body["top_p"] = serde_json::json!(crate::llm::effective_top_p(
            &config.model.default_model,
            config.model.top_p
        ));
    }
    if crate::llm::model_uses_reasoning_effort(&config.model.default_model) {
        body["reasoning_effort"] = serde_json::json!("medium");
    }
    if let Some(thinking) =
        crate::llm::build_thinking_param(&config.model.default_model, config.model.thinking_budget)
    {
        body["thinking"] = thinking;
    }

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!(
            "OpenAI API error ({}): {}",
            status,
            resp.text().await.unwrap_or_default()
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let choice = data["choices"]
        .as_array()
        .and_then(|a| a.first())
        .ok_or_else(|| anyhow::anyhow!("OpenAI API returned empty choices array"))?;
    let finish_reason = choice["finish_reason"]
        .as_str()
        .unwrap_or("stop")
        .to_string();

    let message = &choice["message"];
    let text = message["content"].as_str().unwrap_or("").to_string();

    let reasoning_text = crate::error_classifier::extract_reasoning(message);

    let mut tool_uses = Vec::new();
    if let Some(arr) = message["tool_calls"].as_array() {
        for tc in arr {
            tool_uses.push(crate::llm::ToolCallInfo {
                id: tc["id"].as_str().unwrap_or("").to_string(),
                name: crate::streaming::restore_tool_name_from_api(
                    tc["function"]["name"].as_str().unwrap_or(""),
                ),
                input: tc["function"]["arguments"]
                    .as_str()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or(serde_json::Value::Null),
            });
        }
    }

    let usage = data.get("usage").map(|u| crate::llm::UsageInfo {
        input_tokens: u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        output_tokens: u
            .get("completion_tokens")
            .and_then(|v| v.as_u64())
            .unwrap_or(0),
        cache_read_tokens: None,
        cache_creation_tokens: None,
    });

    Ok((text, tool_uses, finish_reason, usage, reasoning_text))
}

/// 调用Anthropic API进行非流式请求，支持工具调用和思考参数
pub async fn call_anthropic_with_tools(
    client: &Client,
    base_url: &str,
    api_key: &str,
    config: &AppConfig,
    messages: &[serde_json::Value],
    tools: &[ToolDefinition],
) -> Result<ApiResponseInner> {
    let base = base_url.trim_end_matches('/');
    let url = build_api_url(base, "/v1/messages");
    log::info!("[LLM:Anthropic] POST {} (base_input={})", url, base);

    let api_tools: Vec<serde_json::Value> = tools.iter().map(|t| serde_json::json!({
        "name": crate::streaming::sanitize_tool_name_for_api(&t.name), "description": t.description, "input_schema": t.input_schema
    })).collect();

    let system_prompt = messages
        .iter()
        .find(|m| m["role"].as_str() == Some("system"))
        .and_then(|m| m["content"].as_str())
        .unwrap_or("");
    let api_messages: Vec<&serde_json::Value> = messages
        .iter()
        .filter(|m| m["role"].as_str() != Some("system"))
        .collect();

    let mut body = serde_json::json!({
        "model": config.model.default_model,
        "max_tokens": config.model.max_tokens,
        "stream": false,
        "messages": api_messages,
        "tools": api_tools
    });
    if !system_prompt.is_empty() {
        body["system"] = serde_json::json!(system_prompt);
    }
    if !crate::llm::model_ignores_temperature(&config.model.default_model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(
            &config.model.default_model,
            config.model.temperature
        ));
    }
    if !crate::llm::model_ignores_top_p(&config.model.default_model) {
        body["top_p"] = serde_json::json!(crate::llm::effective_top_p(
            &config.model.default_model,
            config.model.top_p
        ));
    }
    if let Some(thinking) =
        crate::llm::build_thinking_param(&config.model.default_model, config.model.thinking_budget)
    {
        body["thinking"] = thinking;
    }

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", &config.api.api_version)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    if !status.is_success() {
        anyhow::bail!(
            "Anthropic API error ({}): {}",
            status,
            resp.text().await.unwrap_or_default()
        );
    }

    let data: serde_json::Value = resp.json().await?;
    let stop_reason = data["stop_reason"]
        .as_str()
        .unwrap_or("end_turn")
        .to_string();

    let mut reasoning_text: Option<String> = None;
    let mut text = String::new();
    if let Some(arr) = data["content"].as_array() {
        for cb in arr {
            match cb["type"].as_str().unwrap_or("") {
                "thinking" => {
                    if let Some(thinking) = cb["thinking"].as_str() {
                        reasoning_text = Some(thinking.to_string());
                    }
                }
                "text" => {
                    if let Some(t) = cb["text"].as_str() {
                        text.push_str(t);
                    }
                }
                _ => {}
            }
        }
    }

    let mut tool_uses = Vec::new();
    if let Some(arr) = data["content"].as_array() {
        for cb in arr {
            if cb["type"].as_str() == Some("tool_use") {
                tool_uses.push(crate::llm::ToolCallInfo {
                    id: cb["id"].as_str().unwrap_or("").to_string(),
                    name: crate::streaming::restore_tool_name_from_api(
                        cb["name"].as_str().unwrap_or(""),
                    ),
                    input: cb["input"].clone(),
                });
            }
        }
    }

    let usage = data.get("usage").map(|u| crate::llm::UsageInfo {
        input_tokens: u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        output_tokens: u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        cache_read_tokens: None,
        cache_creation_tokens: None,
    });

    Ok((text, tool_uses, stop_reason, usage, reasoning_text))
}
