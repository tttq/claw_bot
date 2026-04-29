// Claw Desktop - Agent引擎 - WS层的Agent消息处理
use claw_config::config::AppConfig;
use claw_tools::agent_session;
use crate::ws::server;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

/// Agent任务定义
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTask {
    pub task_id: String,
    pub agent_id: String,
    pub prompt: String,
    pub conversation_id: String,
    #[allow(dead_code)]
    pub context: serde_json::Value,
}

/// Agent任务执行结果
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentTaskResult {
    pub task_id: String,
    pub agent_id: String,
    #[serde(rename = "status")]
    pub status: String,
    pub result: Option<String>,
    pub error: Option<String>,
    #[serde(rename = "durationMs")]
    pub duration_ms: u64,
}

/// 任务执行器trait — 定义异步执行接口
pub trait TaskExecutor: Send + Sync {
    fn execute(&self, task: &AgentTask, config: &AppConfig) -> impl std::future::Future<Output = AgentTaskResult> + Send;
}

/// LLM任务执行器 — 通过LLM流式API执行Agent任务
pub struct LlmTaskExecutor;

impl TaskExecutor for LlmTaskExecutor {
    /// 执行任务 — 发送start/done/error事件，返回执行结果
    async fn execute(&self, task: &AgentTask, config: &AppConfig) -> AgentTaskResult {
        let start = std::time::Instant::now();
        log::info!("[AgentEngine] Executing task={} agent={}", task.task_id, task.agent_id);

        server::emit_subagent_event(&task.task_id, "start", serde_json::json!({
            "task_id": task.task_id,
            "agent_id": task.agent_id,
            "conversation_id": task.conversation_id,
        }));

        match self.run_llm_stream(task, config).await {
            Ok(text) => {
                let duration = start.elapsed().as_millis() as u64;
                server::emit_subagent_event(&task.task_id, "done", serde_json::json!({
                    "task_id": task.task_id,
                    "full_text": text,
                    "duration_ms": duration,
                }));
                AgentTaskResult { task_id: task.task_id.clone(), agent_id: task.agent_id.clone(), status: "completed".into(), result: Some(text), error: None, duration_ms: duration }
            }
            Err(e) => {
                let duration = start.elapsed().as_millis() as u64;
                server::emit_subagent_event(&task.task_id, "error", serde_json::json!({
                    "task_id": task.task_id,
                    "error": e.to_string(),
                    "duration_ms": duration,
                }));
                AgentTaskResult { task_id: task.task_id.clone(), agent_id: task.agent_id.clone(), status: "failed".into(), result: None, error: Some(e.to_string()), duration_ms: duration }
            }
        }
    }
}

impl LlmTaskExecutor {
    /// 运行LLM流式调用 — 根据配置选择OpenAI或Anthropic API
    async fn run_llm_stream(&self, task: &AgentTask, config: &AppConfig) -> Result<String, anyhow::Error> {
        let agent = agent_session::iso_agent_get(task.agent_id.clone())
            .await
            .map_err(|e| anyhow::anyhow!("Agent {} not found: {}", task.agent_id, e))?
            .ok_or_else(|| anyhow::anyhow!("Agent {} does not exist", task.agent_id))?;

        let system_prompt = agent.system_prompt
            .as_ref()
            .filter(|s| !s.is_empty())
            .map(|s| s.as_str())
            .unwrap_or("You are a helpful AI assistant.");

        let model = agent.model_override.as_deref().unwrap_or(&config.model.default_model);
        let api_key = config.resolve_api_key().map_err(|e| anyhow::anyhow!("API key error: {}", e))?;
        let base_url = config.get_base_url();

        if config.is_openai_compatible() {
            self.call_openai(&api_key, base_url, model, system_prompt, &task.prompt, &task.task_id).await
        } else {
            self.call_anthropic(&api_key, base_url, model, system_prompt, &task.prompt, &task.task_id).await
        }
    }

    /// 调用OpenAI流式API — 逐token发送子Agent事件
    async fn call_openai(
        &self, api_key: &str, base_url: &str, model: &str,
        system: &str, user_msg: &str, task_id: &str,
    ) -> Result<String, anyhow::Error> {
        use reqwest::Client;
        let client = Client::builder().timeout(std::time::Duration::from_secs(120)).build()?;
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg}
            ],
            "stream": true,
            "max_tokens": 4096,
        });

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send().await?
            .error_for_status()?;

        let mut all_text = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if json_str == "[DONE]" { break; }
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(content) = v["choices"][0]["delta"]["content"].as_str() {
                            if !content.is_empty() {
                                server::emit_subagent_event(task_id, "token", serde_json::json!({"content": content}));
                                all_text.push_str(content);
                            }
                        }
                    }
                }
            }
        }

        Ok(all_text)
    }

    /// 调用Anthropic流式API — 逐token发送子Agent事件
    async fn call_anthropic(
        &self, api_key: &str, base_url: &str, model: &str,
        system: &str, user_msg: &str, task_id: &str,
    ) -> Result<String, anyhow::Error> {
        use reqwest::Client;
        let client = Client::builder().timeout(std::time::Duration::from_secs(120)).build()?;
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));

        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "system": system,
            "messages": [{"role": "user", "content": user_msg}],
            "stream": true,
        });

        let resp = client.post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send().await?
            .error_for_status()?;

        let mut all_text = String::new();
        let mut stream = resp.bytes_stream();
        while let Some(chunk_result) = stream.next().await {
            let chunk = chunk_result.map_err(|e| anyhow::anyhow!("Stream error: {}", e))?;
            let text = String::from_utf8_lossy(&chunk);
            for line in text.lines() {
                if let Some(json_str) = line.strip_prefix("data: ") {
                    if let Ok(v) = serde_json::from_str::<serde_json::Value>(json_str) {
                        if let Some(content) = v["delta"]["text"].as_str() {
                            if !content.is_empty() {
                                server::emit_subagent_event(task_id, "token", serde_json::json!({"content": content}));
                                all_text.push_str(content);
                            }
                        }
                    }
                }
            }
        }

        Ok(all_text)
    }
}

/// 执行Agent任务 — 验证Agent存在后异步spawn执行，立即返回accepted状态
pub async fn execute_agent_task(task: AgentTask) -> Result<serde_json::Value, String> {
    log::info!("[AgentEngine] Validating agent exists for task={} agent={}", task.task_id, task.agent_id);

    let _agent = match agent_session::iso_agent_get(task.agent_id.clone()).await {
        Ok(Some(a)) => a,
        Ok(None) => {
            return Err(format!("Agent '{}' does not exist. Cannot execute task '{}'.", task.agent_id, task.task_id));
        }
        Err(e) => {
            return Err(format!("Failed to look up agent '{}': {}", task.agent_id, e));
        }
    };

    let config = crate::ws::router::get_config().await;
    let executor = LlmTaskExecutor;

    let task_clone = task.clone();
    tokio::spawn(async move {
        let result = executor.execute(&task_clone, &config).await;
        match &result.status {
            s if s == "completed" || s == "failed" => {
                log::info!("[AgentEngine] task={} completed with status={}", task_clone.task_id, s);
            }
            _ => {}
        }
        let _ = result;
    });

    Ok(serde_json::json!({
        "task_id": task.task_id,
        "agent_id": task.agent_id,
        "status": "accepted",
        "message": "Task submitted for async execution",
    }))
}
