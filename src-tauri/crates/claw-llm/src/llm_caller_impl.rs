// Claw Desktop - LlmCaller 实现 - 复用 claw-llm 的 HTTP 客户端和 API 调用基础设施
// 实现 claw_traits::LlmCaller trait，供 claw-tools 通过 trait 注入调用

use claw_traits::LlmCaller;

pub struct ClawLlmCaller;

#[async_trait::async_trait]
impl LlmCaller for ClawLlmCaller {
/// 单次LLM调用 — 根据is_openai标志选择OpenAI或Anthropic API
    async fn call_once(
        &self,
        api_key: &str,
        base_url: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        is_openai: bool,
    ) -> Result<String, String> {
        let client = crate::llm::http_client();

        if is_openai {
            call_openai_once(client, base_url, api_key, model, system_prompt, user_message).await
        } else {
            call_anthropic_once(client, base_url, api_key, model, system_prompt, user_message).await
        }
    }

    async fn call_once_vision(
        &self,
        api_key: &str,
        base_url: &str,
        model: &str,
        system_prompt: &str,
        user_message: &str,
        image_base64: &str,
        is_openai: bool,
    ) -> Result<String, String> {
        let client = crate::llm::http_client();

        if is_openai {
            call_openai_vision_once(client, base_url, api_key, model, system_prompt, user_message, image_base64).await
        } else {
            call_anthropic_vision_once(client, base_url, api_key, model, system_prompt, user_message, image_base64).await
        }
    }
}

/// 单次OpenAI兼容API调用 — 发送系统提示和用户消息并返回文本内容
async fn call_openai_once(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user_msg: &str,
) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    };

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_msg}
        ],
        "max_tokens": 4096,
    });

    if !crate::llm::model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(model, 0.7));
    }

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("[LlmCaller:OpenAI] Request error: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("[LlmCaller:OpenAI] API error (HTTP {}): {}", status, error_text));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("[LlmCaller:OpenAI] Parse error: {}", e))?;

    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}

/// 单次Anthropic API调用 — 发送系统提示和用户消息并返回文本内容
async fn call_anthropic_once(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user_msg: &str,
) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1/messages") {
        base.to_string()
    } else {
        format!("{}/v1/messages", base)
    };

    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system,
        "messages": [{"role": "user", "content": user_msg}],
    });

    if !crate::llm::model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(model, 0.7));
    }

    if let Some(thinking) = crate::llm::build_thinking_param(model, 4096) {
        body["thinking"] = thinking;
    }

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("[LlmCaller:Anthropic] Request error: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("[LlmCaller:Anthropic] API error (HTTP {}): {}", status, error_text));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("[LlmCaller:Anthropic] Parse error: {}", e))?;

    let content = v["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}

/// 注册LLM调用器到全局注入点
pub fn register_llm_caller() {
    let caller: std::sync::Arc<dyn LlmCaller> = std::sync::Arc::new(ClawLlmCaller);
    claw_traits::set_llm_caller(caller);
    log::info!("[LlmCaller] Registered to global injection point");
}

async fn call_openai_vision_once(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user_msg: &str,
    image_base64: &str,
) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") {
        base.to_string()
    } else {
        format!("{}/chat/completions", base)
    };

    let user_content = serde_json::json!([
        {"type": "text", "text": user_msg},
        {"type": "image_url", "image_url": {"url": format!("data:image/png;base64,{}", image_base64)}}
    ]);

    let mut body = serde_json::json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system},
            {"role": "user", "content": user_content}
        ],
        "max_tokens": 4096,
    });

    if !crate::llm::model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(model, 0.7));
    }

    let resp = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("[LlmCaller:OpenAI:Vision] Request error: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("[LlmCaller:OpenAI:Vision] API error (HTTP {}): {}", status, error_text));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("[LlmCaller:OpenAI:Vision] Parse error: {}", e))?;

    let content = v["choices"][0]["message"]["content"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}

async fn call_anthropic_vision_once(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    system: &str,
    user_msg: &str,
    image_base64: &str,
) -> Result<String, String> {
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1/messages") {
        base.to_string()
    } else {
        format!("{}/v1/messages", base)
    };

    let user_content = serde_json::json!([
        {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": image_base64}},
        {"type": "text", "text": user_msg}
    ]);

    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": 4096,
        "system": system,
        "messages": [{"role": "user", "content": user_content}],
    });

    if !crate::llm::model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(crate::llm::effective_temperature(model, 0.7));
    }

    if let Some(thinking) = crate::llm::build_thinking_param(model, 4096) {
        body["thinking"] = thinking;
    }

    let resp = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("[LlmCaller:Anthropic:Vision] Request error: {}", e))?;

    let status = resp.status();
    if !status.is_success() {
        let error_text = resp.text().await.unwrap_or_default();
        return Err(format!("[LlmCaller:Anthropic:Vision] API error (HTTP {}): {}", status, error_text));
    }

    let v: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| format!("[LlmCaller:Anthropic:Vision] Parse error: {}", e))?;

    let content = v["content"][0]["text"]
        .as_str()
        .unwrap_or("")
        .to_string();

    Ok(content)
}
