// Claw Desktop - LLM主逻辑 - 模型调用、消息构建、响应解析
// 类型定义 + 公共API入口 + 辅助函数
// 核心逻辑已拆分到子模块: constants, loop_detector, api_client, streaming, prompt_builder, tool_loop

use serde::{Serialize, Deserialize};
use sea_orm::{EntityTrait, ColumnTrait, QueryFilter};

pub use super::streaming::{set_ws_emit_callback, emit_chat_stream};

static HTTP_CLIENT: std::sync::OnceLock<reqwest::Client> = std::sync::OnceLock::new();
static CANCEL_FLAGS: std::sync::OnceLock<dashmap::DashMap<String, bool>> = std::sync::OnceLock::new();

/// 获取HTTP客户端单例
pub fn http_client() -> &'static reqwest::Client {
    HTTP_CLIENT.get_or_init(|| {
        reqwest::Client::builder()
            .pool_max_idle_per_host(20)
            .timeout(std::time::Duration::from_secs(300))
            .connect_timeout(std::time::Duration::from_secs(30))
            .build()
            .expect("[LLM:llm] Failed to build HTTP client")
    })
}

/// 获取取消标志映射表单例
fn cancel_flags() -> &'static dashmap::DashMap<String, bool> {
    CANCEL_FLAGS.get_or_init(|| dashmap::DashMap::new())
}

/// 按字节截断字符串，确保不截断在UTF-8字符中间
fn truncate_str(s: &str, max_bytes: usize) -> &str {
    if max_bytes >= s.len() { return s; }
    let mut end = max_bytes;
    while end > 0 && !s.is_char_boundary(end) { end -= 1; }
    &s[..end]
}

pub struct ChatContext {
    pub agent_id: Option<String>,
    pub agent_max_turns: usize,
    pub rag_context: String,
    pub history: Vec<ApiMessage>,
    pub tools: Vec<ToolDefinition>,
    pub system_prompt: String,
    pub messages_for_api: Vec<serde_json::Value>,
}

/// 准备聊天上下文 — 加载历史消息、构建RAG上下文、收集工具定义、生成系统提示词
pub async fn prepare_chat_context(
    config: &AppConfig,
    conversation_id: &str,
    user_message: &str,
    images: Option<&[serde_json::Value]>,
) -> Result<ChatContext> {
    let _api_key = config.resolve_api_key()?;
    let _base_url = config.get_base_url();
    let agent_id = extract_agent_id(conversation_id).await;

    let agent_max_turns: usize = if let Some(ref aid) = agent_id {
        if let Some(agent_db) = claw_db::db::try_get_agent_db() {
            if let Ok(Some(agent)) = claw_db::db::agent_entities::agents::Entity::find_by_id(aid.to_string()).one(agent_db).await {
                agent.max_turns as usize
            } else { MAX_TOOL_ROUNDS }
        } else { MAX_TOOL_ROUNDS }
    } else { MAX_TOOL_ROUNDS };

    let _ = claw_rag::rag::compact_conversation_if_needed(
        conversation_id, agent_id.as_deref(), &config.model.default_model, Some(config.advanced.auto_compact_tokens)
    ).await;

    let rag_context = claw_rag::rag::build_rag_context(agent_id.as_deref(), conversation_id, user_message).await.unwrap_or_default();

    let ctx_window = claw_rag::rag::get_model_context_window(&config.model.default_model);
    let history_budget: usize = ((ctx_window as f64) * 0.55) as usize;
    let rag_tokens = (rag_context.len() / 4) as usize;

    let all_msgs = match claw_db::database::Database::get_messages(conversation_id).await {
        Ok(msgs) => msgs.iter()
            .filter(|m| m.role == "user" || m.role == "assistant" || m.role == "tool")
            .map(|m| ApiMessage { role: m.role.clone(), content: m.content.clone() })
            .collect::<Vec<_>>(),
        Err(_) => Vec::new(),
    };

    let base_tokens = (rag_context.len() / 4) + 500;
    let effective_budget = history_budget.saturating_sub(rag_tokens);
    let mut used_tokens = base_tokens;
    let mut history = Vec::new();
    for msg in all_msgs.iter().rev() {
        let msg_tokens = msg.content.len() / 4;
        if used_tokens + msg_tokens > effective_budget { break; }
        used_tokens += msg_tokens;
        history.insert(0, msg.clone());
    }
    log::info!("[LLM] Loaded {} history messages ({}/{} tokens budget, RAG {} chars)", history.len(), used_tokens, history_budget, rag_context.len());

    let tools_raw: Vec<ToolDefinition> = if let Some(ref aid) = agent_id {
        if let Some(executor) = crate::tool_executor::get_tool_executor() {
            executor.list_tools_for_agent(aid).await
        } else {
            log::warn!("[LLM] ToolExecutor not registered, returning empty tool list");
            Vec::new()
        }
    } else {
        if let Some(executor) = crate::tool_executor::get_tool_executor() {
            executor.list_all_tools()
        } else {
            log::warn!("[LLM] ToolExecutor not registered, returning empty tool list");
            Vec::new()
        }
    };
    let tool_count = tools_raw.len();
    let tool_catalog = build_tool_catalog(&tools_raw);
    let system_prompt = claw_rag::rag::build_system_prompt_with_agent(config, agent_id.as_deref(), Some(agent_max_turns), tool_count, Some(&tool_catalog)).await;
    let tools: Vec<ToolDefinition> = tools_raw.iter().map(|t| ToolDefinition { name: t.name.clone(), description: t.description.clone(), input_schema: t.input_schema.clone(), category: t.category.clone(), tags: t.tags.clone() }).collect();
    log::info!("[LLM] {} tools registered | Provider: {} | Model: {} | MaxTurns: {}", tools.len(), config.model.provider, config.model.default_model, agent_max_turns);

    let mut messages_for_api = PromptBuilder::build_messages(&history, &system_prompt, &rag_context, user_message);

    inject_images_into_messages(&mut messages_for_api, images);

    Ok(ChatContext { agent_id, agent_max_turns, rag_context, history, tools, system_prompt, messages_for_api })
}

/// 将图片和文件附件注入到最后一条用户消息中
fn inject_images_into_messages(messages: &mut Vec<serde_json::Value>, images: Option<&[serde_json::Value]>) {
    if let Some(attachments) = images {
        if attachments.is_empty() { return; }
        if let Some(last_msg) = messages.last_mut() {
            if last_msg.get("role").and_then(|v| v.as_str()) != Some("user") { return; }
            let mut content_parts: Vec<serde_json::Value> = Vec::new();
            let text_content = last_msg.get("content").and_then(|v| v.as_str()).unwrap_or("");
            content_parts.push(serde_json::json!({"type": "text", "text": text_content}));
            let mut other_media_info: Vec<String> = Vec::new();
            for att in attachments {
                let data_url = att.get("data_url").and_then(|v| v.as_str()).unwrap_or("");
                let media_type = att.get("media_type").and_then(|v| v.as_str()).unwrap_or("application/octet-stream");
                let name = att.get("name").and_then(|v| v.as_str()).unwrap_or("unknown");
                if data_url.is_empty() { continue; }
                let b64 = if data_url.contains(',') { data_url.splitn(2, ',').nth(1).unwrap_or("") } else { data_url };
                if b64.is_empty() { continue; }
                if media_type.starts_with("image/") {
                    content_parts.push(serde_json::json!({
                        "type": "image_url",
                        "image_url": { "url": format!("data:{};base64,{}", media_type, b64) }
                    }));
                } else {
                    let size_kb = b64.len() * 3 / 4 / 1024;
                    other_media_info.push(format!("[File: {} | Type: {} | ~{}KB]", name, media_type, size_kb));
                }
            }
            if !other_media_info.is_empty() {
                if let Some(text_part) = content_parts.iter_mut().find(|p| p["type"] == "text") {
                    if let Some(t) = text_part.get_mut("text") {
                        *t = serde_json::Value::String(format!("{}\n\n--- Attachments ({} files) ---\n{}\n--- End ---",
                            t.as_str().unwrap_or(""), other_media_info.len(), other_media_info.join("\n")));
                    }
                }
            }
            if content_parts.len() > 1 {
                last_msg["content"] = serde_json::json!(content_parts);
            }
        }
    }
}

/// 根据模型名称返回有效温度 — 推理模型(o1/o3等)使用固定温度
pub fn effective_temperature(model: &str, configured_temp: f64) -> f64 {
    let m = model.to_lowercase();

    if m.contains("kimi-k2") {
        return 1.0;
    }
    if m.contains("deepseek-r1") || m.contains("deepseek_reasoner") || m.contains("deepseek-r") {
        return 1.0;
    }
    if m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4") {
        return 1.0;
    }
    if m.contains("claude-opus-4.7") || m.contains("claude-opus-4-7") {
        return 1.0;
    }
    if m.contains("qwen3") && m.contains("think") {
        return 0.6;
    }
    if m.contains("qwq") {
        return 0.6;
    }
    if m.contains("gemini-2.5") || m.contains("gemini-3") {
        return 1.0;
    }
    if m.contains("minimax-m2") {
        return 1.0;
    }

    configured_temp
}

/// 根据模型名称返回有效top_p — 推理模型使用固定top_p
pub fn effective_top_p(model: &str, configured_top_p: f64) -> f64 {
    let m = model.to_lowercase();

    if m.contains("kimi-k2") {
        return 0.95;
    }
    if m.contains("claude-opus-4.7") || m.contains("claude-opus-4-7") {
        return 1.0;
    }
    if m.contains("qwen3") && m.contains("think") {
        return 0.95;
    }
    if m.contains("qwq") {
        return 0.95;
    }
    if m.contains("gemini-2.5") || m.contains("gemini-3") {
        return 0.95;
    }
    if m.contains("minimax-m2") {
        return 0.95;
    }

    configured_top_p
}

/// 判断模型是否忽略温度参数
pub fn model_ignores_temperature(model: &str) -> bool {
    let m = model.to_lowercase();
    m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
        || m.contains("claude-opus-4.7") || m.contains("claude-opus-4-7")
}

/// 判断模型是否忽略top_p参数
pub fn model_ignores_top_p(model: &str) -> bool {
    let m = model.to_lowercase();
    m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
        || m.contains("claude-opus-4.7") || m.contains("claude-opus-4-7")
}

/// 判断模型是否支持思考(thinking)模式
pub fn model_supports_thinking(model: &str) -> bool {
    let m = model.to_lowercase();
    m.contains("kimi-k2")
        || m.contains("claude-3.5-sonnet")
        || m.contains("claude-3.7")
        || m.contains("claude-4")
        || m.contains("claude-sonnet-4")
        || m.contains("claude-opus-4")
        || m.contains("claude-haiku-4")
        || m.contains("deepseek-r1")
        || m.contains("deepseek_reasoner")
        || m.contains("deepseek-r")
        || m.contains("qwen3")
        || m.contains("qwq")
        || m.contains("gemini-2.5")
        || m.contains("gemini-3")
        || m.contains("gemini-3.")
        || m.contains("o1") || m.contains("o3") || m.contains("o4")
        || m.contains("gpt-5")
        || m.contains("glm-5")
        || m.contains("glm-4.7")
        || m.contains("minimax-m2")
        || m.contains("mimo-v2-pro")
        || m.contains("mimo-v2-omni")
        || m.contains("grok-4-fast-reasoning")
        || m.contains("grok-4.1-fast-reasoning")
        || m.contains("grok-4-1-fast-reasoning")
        || m.contains("grok-4.20")
        || m.contains("grok-code-fast")
        || m.contains("sonar-reasoning")
        || m.contains("ernie-5.0-thinking")
        || m.contains("kimi-k2-thinking")
        || m.contains("kimi-k2.5")
        || m.contains("kimi-code")
        || m.contains("deepseek-v3.2")
        || m.contains("deepseek-v3-2")
        || m.contains("kilo/auto")
}

/// 根据模型构建思考参数配置 — budget_tokens/type/等
pub fn build_thinking_param(model: &str, thinking_budget: u64) -> Option<serde_json::Value> {
    if !model_supports_thinking(model) {
        return None;
    }

    let m = model.to_lowercase();

    if m.contains("kimi-k2") {
        return Some(serde_json::json!({"type": "enabled"}));
    }

    if m.contains("claude-opus-4.7") || m.contains("claude-opus-4-7") {
        return Some(serde_json::json!({"type": "adaptive"}));
    }

    if m.contains("claude") {
        return Some(serde_json::json!({
            "type": "enabled",
            "budget_tokens": thinking_budget.max(1024)
        }));
    }

    if m.contains("deepseek-r1") || m.contains("deepseek_reasoner") || m.contains("deepseek-r") {
        return None;
    }

    if m.contains("qwen3") || m.contains("qwq") {
        return None;
    }

    if m.contains("gemini") {
        return None;
    }

    if m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4") {
        return None;
    }

    if m.contains("gpt-5") {
        return None;
    }

    if m.contains("glm") || m.contains("minimax") || m.contains("mimo") {
        return None;
    }

    if m.contains("grok") {
        return None;
    }

    if m.contains("sonar-reasoning") || m.contains("ernie") || m.contains("kimi") || m.contains("kilo") {
        return None;
    }

    if m.contains("deepseek-v3.2") || m.contains("deepseek-v3-2") {
        return None;
    }

    None
}

/// 判断模型是否使用reasoning_effort参数（替代thinking budget）
pub fn model_uses_reasoning_effort(model: &str) -> bool {
    let m = model.to_lowercase();
    m.starts_with("o1") || m.starts_with("o3") || m.starts_with("o4")
}

pub(crate) fn build_api_url(base_url: &str, endpoint: &str) -> String {
    let base = base_url.trim_end_matches('/');
    let ep = endpoint.trim_start_matches('/');
    if base.ends_with(ep) { base.to_string() } else { format!("{}/{}", base, ep) }
}

/// 请求取消指定会话的LLM调用
pub fn request_cancel(conversation_id: &str) {
    cancel_flags().insert(conversation_id.to_string(), true);
}

/// 检查指定会话是否已被标记为取消
pub fn is_cancelled(conversation_id: &str) -> bool {
    cancel_flags().get(conversation_id).map(|v| *v).unwrap_or(false)
}

/// 清除指定会话的取消标志
pub fn clear_cancel(conversation_id: &str) {
    cancel_flags().remove(conversation_id);
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChatResponse {
    pub text: String,
    pub usage: Option<UsageInfo>,
    pub tool_calls: Vec<ToolCallInfo>,
    pub tool_executions: Vec<ToolExecutionInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UsageInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallInfo {
    pub id: String,
    pub name: String,
    pub input: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExecutionInfo {
    pub round: usize,
    pub tool_name: String,
    pub tool_input: serde_json::Value,
    pub tool_result: String,
    pub duration_ms: u128,
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct TestLogEntry {
    pub timestamp: String,
    pub level: String,
    pub phase: String,
    pub detail: String,
}

/// 从会话ID提取关联的Agent ID
async fn extract_agent_id(conversation_id: &str) -> Option<String> {
    if let Some(agent_db) = claw_db::db::try_get_agent_db() {
        if let Ok(sessions) = claw_db::db::agent_entities::agent_sessions::Entity::find()
            .filter(claw_db::db::agent_entities::agent_sessions::Column::ConversationId.eq(conversation_id))
            .all(agent_db).await
        {
            return sessions.into_iter().next().map(|s| s.agent_id);
        }
    }
    None
}

/// 执行指定工具并返回JSON格式的结果字符串
pub async fn execute_tool(name: &str, input: &serde_json::Value) -> String {
    if let Some(executor) = crate::tool_executor::get_tool_executor() {
        match executor.execute(name, input).await {
            Ok(result) => serde_json::to_string(&result).unwrap_or_else(|_| r#"{"status":"ok"}"#.to_string()),
            Err(e) => {
                log::error!("[LLM] Tool '{}' execution failed: {}", name, e);
                serde_json::json!({"error": e, "tool_name": name}).to_string()
            }
        }
    } else {
        log::error!("[LLM] ToolExecutor not registered, cannot execute tool '{}'", name);
        serde_json::json!({"error": "ToolExecutor not initialized. Call claw_tools::tool_executor::create_and_register_tool_executor() at startup", "tool_name": name}).to_string()
    }
}

/// 构建工具分类目录 — 按类型分组展示可用工具名称和描述
fn build_tool_catalog(tools: &[ToolDefinition]) -> String {
    if tools.is_empty() {
        return "## Tool Catalog\nNo tools currently available.".to_string();
    }

    let mut catalog = String::from("## Tool Catalog\nThe following tools are available for use:\n\n");

    let mut file_tools: Vec<&ToolDefinition> = Vec::new();
    let mut shell_tools: Vec<&ToolDefinition> = Vec::new();
    let mut search_tools: Vec<&ToolDefinition> = Vec::new();
    let mut web_tools: Vec<&ToolDefinition> = Vec::new();
    let mut git_tools: Vec<&ToolDefinition> = Vec::new();
    let mut agent_tools: Vec<&ToolDefinition> = Vec::new();
    let mut browser_tools: Vec<&ToolDefinition> = Vec::new();
    let mut skill_tools: Vec<&ToolDefinition> = Vec::new();
    let mut other_tools: Vec<&ToolDefinition> = Vec::new();

    for tool in tools {
        let name = tool.name.to_lowercase();
        if name.starts_with("skill:") || name.starts_with("skill_") {
            skill_tools.push(tool);
        } else if ["read", "edit", "write", "file_read", "file_edit", "file_write"].contains(&name.as_str()) {
            file_tools.push(tool);
        } else if ["bash", "bash_cancel"].contains(&name.as_str()) {
            shell_tools.push(tool);
        } else if ["glob", "grep", "tool_search"].contains(&name.as_str()) {
            search_tools.push(tool);
        } else if ["web_fetch", "web_search", "fetch", "search"].contains(&name.as_str()) {
            web_tools.push(tool);
        } else if name.starts_with("git_") || name.starts_with("git") {
            git_tools.push(tool);
        } else if name.starts_with("browser_") || name.starts_with("browser") {
            browser_tools.push(tool);
        } else if ["agent", "todo_write", "todo_get", "task_create", "task_list", "workflow", "skill",
                    "enter_plan_mode", "exit_plan_mode", "brief", "config", "notebook_edit",
                    "schedule_cron", "ask_user_question"].contains(&name.as_str()) {
            agent_tools.push(tool);
        } else {
            other_tools.push(tool);
        }
    }

    let mut add_section = |title: &str, tools: &[&ToolDefinition]| {
        if tools.is_empty() { return; }
        catalog.push_str(&format!("### {}\n", title));
        for tool in tools {
            let short_desc: String = tool.description.chars().take(120).collect();
            let api_name = crate::streaming::sanitize_tool_name_for_api(&tool.name);
            catalog.push_str(&format!("- **{}**: {}\n", api_name, short_desc));
        }
        catalog.push('\n');
    };

    add_section("File Operations", &file_tools);
    add_section("Shell & Execution", &shell_tools);
    add_section("Search & Discovery", &search_tools);
    add_section("Web & Network", &web_tools);
    add_section("Git & Version Control", &git_tools);
    add_section("Agent & Task Management", &agent_tools);
    add_section("Browser Automation", &browser_tools);
    add_section("Skills (use `Skill_skill-name` to invoke)", &skill_tools);
    add_section("Other Tools", &other_tools);

    catalog.push_str("---\n*Tool catalog is dynamically updated. Use `ToolSearch` to discover tools by keyword.*\n");
    catalog
}

/// 将用户交互存储到RAG记忆系统
pub async fn store_interaction_to_rag(conv_id: &str, agent_id: Option<&str>, query: &str, response: &str) -> Result<(), String> {
    if let Some(aid) = agent_id {
        let user_summary = summarize_for_memory(query, "user");
        claw_rag::rag::store_enhanced_memory(
            aid, Some(conv_id), &user_summary, "observation", "conversation_summary", None, None
        ).await?;

        let assistant_summary = summarize_for_memory(response, "assistant");
        let fact_type = classify_memory_type(response);
        claw_rag::rag::store_enhanced_memory(
            aid, Some(conv_id), &assistant_summary, &fact_type, "conversation_summary", None, None
        ).await?;

        extract_world_facts(aid, conv_id, query, response).await.map_err(|e| {
            log::warn!("[LLM:store_interaction_to_rag] extract_world_facts failed: {}", e);
            e
        }).ok();
    }
    Ok(())
}

/// 为记忆存储生成文本摘要 — 截断过长文本并添加角色标记
pub fn summarize_for_memory(text: &str, role: &str) -> String {
    if text.is_empty() { return String::new(); }

    let max_len = 300;
    if text.len() <= max_len { return text.to_string(); }

    let sentences: Vec<&str> = text.split(|c: char| c == '.' || c == '。' || c == '\n')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect();

    if sentences.is_empty() {
        let mut end = max_len;
        while end > 0 && !text.is_char_boundary(end) { end -= 1; }
        return text[..end].to_string();
    }

    let mut summary = String::new();
    for s in &sentences {
        if summary.len() + s.len() + 2 > max_len { break; }
        if !summary.is_empty() { summary.push_str(". "); }
        summary.push_str(s);
    }

    if summary.is_empty() {
        let mut end = max_len;
        while end > 0 && !text.is_char_boundary(end) { end -= 1; }
        summary = text[..end].to_string();
    }

    format!("[{}] {}", role, summary)
}

/// 根据文本内容分类记忆类型 — observation/preference/procedure/fact
fn classify_memory_type(text: &str) -> String {
    let lower = text.to_lowercase();
    let factual_indicators = ["is ", "are ", "was ", "were ", "means ", "refers to ", "defined as ",
        "stands for ", "consists of ", "contains ", "supports ", "provides ", "enables ",
        "的原理", "的定义", "的作用", "的功能", "的架构", "的组成"];
    let experience_indicators = ["i ", "we ", "tried ", "found ", "discovered ", "learned ",
        "solved ", "fixed ", "implemented ", "created ", "built ",
        "我尝试", "我发现", "我解决", "我实现", "我修复", "我创建"];
    let mental_model_indicators = ["pattern", "best practice", "rule", "principle", "strategy",
        "approach", "methodology", "workflow", "convention",
        "模式", "最佳实践", "原则", "策略", "方法", "规范"];

    for indicator in &mental_model_indicators {
        if lower.contains(indicator) { return "mental_model".to_string(); }
    }
    for indicator in &factual_indicators {
        if lower.contains(indicator) { return "world".to_string(); }
    }
    for indicator in &experience_indicators {
        if lower.contains(indicator) { return "experience".to_string(); }
    }
    "experience".to_string()
}

/// 从对话内容中提取世界知识事实并存储到RAG
async fn extract_world_facts(agent_id: &str, conv_id: &str, query: &str, response: &str) -> Result<(), String> {
    let combined = format!("{}\n{}", query, response);
    let lower = combined.to_lowercase();

    let fact_patterns: &[(&str, &str)] = &[
        ("uses ", "tech_stack"), ("built with ", "tech_stack"), ("powered by ", "tech_stack"),
        ("使用", "tech_stack"), ("基于", "tech_stack"), ("采用", "tech_stack"),
        ("prefers ", "preference"), ("likes ", "preference"), ("always ", "preference"),
        ("喜欢", "preference"), ("偏好", "preference"), ("习惯", "preference"),
        ("works at ", "workplace"), ("project is ", "project"), ("repository is ", "project"),
        ("在", "workplace"), ("项目是", "project"),
    ];

    for (pattern, category) in fact_patterns {
        if let Some(pos) = lower.find(pattern) {
            let start = pos;
            let end_byte = combined.char_indices()
                .skip_while(|(i, _)| *i < start)
                .take(150)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(combined.len());
            let fact_text = combined[start..end_byte].lines().next().unwrap_or("").trim().to_string();
            if !fact_text.is_empty() && fact_text.chars().count() > 3 {
                claw_rag::rag::store_enhanced_memory(
                    agent_id, Some(conv_id), &fact_text, "world", category, None, None
                ).await.map_err(|e| {
                    log::warn!("[LLM:extract_world_facts] store_enhanced_memory failed: {}", e);
                    e
                }).ok();
            }
        }
    }

    Ok(())
}

use claw_config::config::AppConfig;
use claw_types::common::ToolDefinition;
use anyhow::Result;
use super::prompt_builder::{ApiMessage, PromptBuilder};
use super::tool_loop::execute_tool_loop;
use super::constants::*;

/// 发送聊天消息（非流式）— 执行完整的工具循环
pub async fn send_chat_message(
    config: &AppConfig,
    conversation_id: &str,
    user_message: &str,
    images: Option<&[serde_json::Value]>,
) -> Result<ChatResponse> {
    log::info!("[LLM] ========== send_chat_message START (Harness v3 with RAG+Profile) ==========");
    log::info!("[LLM] conversation_id: {}, user_msg_len: {}", truncate_str(conversation_id, 16), user_message.len());

    let ctx = prepare_chat_context(config, conversation_id, user_message, images).await?;

    let (final_text, all_tool_calls, all_tool_executions, final_usage) = execute_tool_loop(
        config, conversation_id, user_message, &mut ctx.messages_for_api.clone(), &ctx.tools, None, false, Some(ctx.agent_max_turns),
    ).await.map_err(|e| anyhow::anyhow!(e))?;

    store_interaction_to_rag(conversation_id, ctx.agent_id.as_deref(), user_message, &final_text).await.map_err(|e| {
        log::warn!("[LLM:send_chat_message] store_interaction_to_rag failed: {}", e);
        e
    }).ok();
    if let Some(aid) = ctx.agent_id {
        claw_rag::rag::update_user_profile(&aid, user_message, &final_text).await.map_err(|e| {
            log::warn!("[LLM:send_chat_message] update_user_profile failed: {}", e);
            e
        }).ok();
    }
    log::info!("[LLM] ========== END | text={} chars, tools={}, executions={} ==========",
        final_text.len(), all_tool_calls.len(), all_tool_executions.len());

    Ok(ChatResponse { text: final_text, usage: final_usage, tool_calls: all_tool_calls, tool_executions: all_tool_executions })
}

/// 发送聊天消息（流式）— 实时推送工具循环过程
pub async fn send_chat_message_streaming(
    config: &AppConfig,
    conversation_id: &str,
    user_message: &str,
    app_handle: tauri::AppHandle,
    images: Option<&[serde_json::Value]>,
) -> Result<(String, Option<UsageInfo>)> {
    let start_time = std::time::Instant::now();
    const TOTAL_REQUEST_TIMEOUT_SECS: u64 = 180;
    log::info!("[LLM:Stream] ========== Streaming START (real-time tool loop) ==========");
    log::info!("[LLM:Stream] conv={}, msg_len={}, model={}, timeout={}s",
        truncate_str(conversation_id, 16),
        user_message.len(),
        config.model.default_model,
        TOTAL_REQUEST_TIMEOUT_SECS
    );

    clear_cancel(conversation_id);

    if let Err(e) = super::streaming::emit_chat_stream(&app_handle, serde_json::json!({"type": "start", "conversation_id": conversation_id})) {
        log::error!("[LLM:Stream] Failed to emit 'start' event: {}", e);
        let _ = super::streaming::emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": format!("Failed to start stream: {}", e)}));
        return Err(anyhow::anyhow!("Failed to emit start event: {}", e));
    }

    let ctx = match prepare_chat_context(config, conversation_id, user_message, images).await {
        Ok(c) => c,
        Err(e) => {
            let err_msg = format!("Context preparation failed: {}", e);
            log::error!("[LLM:Stream] {}", err_msg);
            let _ = super::streaming::emit_chat_stream(&app_handle, serde_json::json!({"type": "error", "conversation_id": conversation_id, "content": err_msg.clone()}));
            return Err(e);
        }
    };

    let (all_text, _all_tool_calls, all_tool_executions, final_usage) = execute_tool_loop(
        config, conversation_id, user_message, &mut ctx.messages_for_api.clone(), &ctx.tools, Some(&app_handle), true, Some(ctx.agent_max_turns),
    ).await.map_err(|e| anyhow::anyhow!(e))?;

    store_interaction_to_rag(conversation_id, ctx.agent_id.as_deref(), user_message, &all_text).await.map_err(|e| {
        log::warn!("[LLM:stream_chat_message] store_interaction_to_rag failed: {}", e);
        e
    }).ok();
    if let Some(aid) = ctx.agent_id {
        claw_rag::rag::update_user_profile(&aid, user_message, &all_text).await.map_err(|e| {
            log::warn!("[LLM:stream_chat_message] update_user_profile failed: {}", e);
            e
        }).ok();
    }

    let tool_exec_summary: Vec<serde_json::Value> = all_tool_executions.iter().map(|te| {
        let result_preview: String = te.tool_result.chars().take(1000).collect();
        serde_json::json!({
            "round": te.round,
            "tool_name": te.tool_name,
            "tool_input": serde_json::to_string(&te.tool_input).unwrap_or_default().chars().take(500).collect::<String>(),
            "tool_result": result_preview,
            "duration_ms": te.duration_ms,
        })
    }).collect();

    if let Err(e) = super::streaming::emit_chat_stream(&app_handle, serde_json::json!({
        "type": "done",
        "conversation_id": conversation_id,
        "full_text": &all_text,
        "tool_executions": tool_exec_summary
    })) {
        log::error!("[LLM:Stream] Failed to emit 'done' event: {}", e);
    }
    clear_cancel(conversation_id);

    let elapsed = start_time.elapsed();
    log::info!("[LLM:Stream] ========== END | text={} chars, executions={}, elapsed={:.2}s ==========",
        all_text.len(),
        all_tool_executions.len(),
        elapsed.as_secs_f64()
    );

    Ok((all_text, final_usage))
}

/// 测试LLM连接 — 返回详细的诊断日志和结果
pub async fn test_llm_connection_detailed(config: &claw_config::config::AppConfig) -> Result<serde_json::Value, String> {
    let mut logs: Vec<TestLogEntry> = Vec::new();
    let now = || chrono::Local::now().format("%H:%M:%S%.3f").to_string();

    macro_rules! log_entry {
        ($level:expr, $phase:expr, $detail:expr) => {
            logs.push(TestLogEntry {
                timestamp: now(),
                level: $level.to_string(),
                phase: $phase.to_string(),
                detail: $detail.to_string(),
            });
        };
    }

    log_entry!("INFO", "Init", "Starting LLM connection test...");
    log_entry!("INFO", "Config", format!("Provider: {}", config.model.provider));
    log_entry!("INFO", "Config", format!("Default Model: {}", config.model.default_model));

    log_entry!("INFO", "APIKey", "Resolving API key...");
    let api_key = match config.resolve_api_key() {
        Ok(key) => {
            if key.is_empty() {
                log_entry!("ERROR", "APIKey", "Resolved API key is empty! Check Model settings or env vars.");
                return Ok(serde_json::json!({ "success": false, "message": "API Key must not be empty (configure in Model settings)", "logs": logs }));
            }
            let source = if !config.model.custom_api_key.is_empty() { "Model settings" } else { "env/config fallback" };
            log_entry!("INFO", "APIKey", format!("API key resolved ({} chars, source: {})", key.len(), source));
            key
        },
        Err(e) => {
            log_entry!("ERROR", "APIKey", format!("Failed to resolve: {}", e));
            return Ok(serde_json::json!({ "success": false, "message": e.to_string(), "logs": logs }));
        }
    };

    log_entry!("INFO", "URL", "Resolving base URL...");
    let base_url = config.get_base_url().to_string();
    if base_url.is_empty() {
        log_entry!("ERROR", "URL", "Base URL is empty!");
        return Ok(serde_json::json!({ "success": false, "message": "Model URL must not be empty", "logs": logs }));
    }
    log_entry!("INFO", "URL", format!("Base URL: {}", base_url));

    let model_name = if config.model.provider == "custom" {
        if config.model.custom_model_name.is_empty() {
            log_entry!("ERROR", "Model", "Custom model name is empty!");
            return Ok(serde_json::json!({ "success": false, "message": "Model Name must not be empty", "logs": logs }));
        }
        log_entry!("INFO", "Model", format!("Custom Model: {}", config.model.custom_model_name));
        config.model.custom_model_name.clone()
    } else {
        log_entry!("INFO", "Model", format!("Model: {}", config.model.default_model));
        config.model.default_model.clone()
    };

    log_entry!("INFO", "HTTPClient", format!("Creating HTTP client (timeout={}s)...", config.api.timeout_seconds));
    let client_result = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(config.api.timeout_seconds))
        .danger_accept_invalid_certs(false)
        .build();
    let client = match client_result {
        Ok(c) => { log_entry!("INFO", "HTTPClient", "HTTP client created successfully"); c }
        Err(e) => { log_entry!("ERROR", "HTTPClient", format!("Failed to create HTTP client: {}", e)); return Ok(serde_json::json!({ "success": false, "message": e.to_string(), "logs": logs })); }
    };

    let test_payload = "Hello! This is a connectivity test from Claw Desktop. Please respond briefly.";
    let result = if config.is_openai_compatible() {
        test_openai_connection_detailed(&client, &base_url, &api_key, &model_name, test_payload, &mut logs).await
    } else {
        test_anthropic_connection_detailed(&client, &base_url, &api_key, &model_name, &config.api.api_version, test_payload, &mut logs).await
    };

    match result {
        Ok(response_data) => {
            log_entry!("SUCCESS", "Result", "Connection test completed successfully!");
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Connection OK! Model: {}, response normal", response_data.get("model").and_then(|v| v.as_str()).unwrap_or(&model_name)),
                "logs": logs,
                "response": response_data,
            }))
        }
        Err(e) => {
            log_entry!("ERROR", "Result", format!("Test failed: {}", e));
            Ok(serde_json::json!({ "success": false, "message": e.to_string(), "logs": logs }))
        }
    }
}

/// 测试Anthropic API连接 — 记录详细的请求和响应诊断日志
async fn test_anthropic_connection_detailed(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    api_version: &str,
    test_message: &str,
    logs: &mut Vec<TestLogEntry>,
) -> Result<serde_json::Value, anyhow::Error> {
    let now = || chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/v1/messages") { base.to_string() } else { format!("{}/v1/messages", base) };
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Request".into(), detail: format!("Target URL: {}", url) });
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Request".into(), detail: format!("API Version: {}", api_version) });

    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": 64,
        "messages": [{"role": "user", "content": test_message}]
    });
    if !model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(effective_temperature(model, 0.0));
    }
    if let Some(thinking) = build_thinking_param(model, 4096) {
        body["thinking"] = thinking;
    }
    logs.push(TestLogEntry { timestamp: now(), level: "DEBUG".into(), phase: "Request".into(), detail: format!("Request body: {}", serde_json::to_string_pretty(&body).unwrap_or_default()) });

    let start = std::time::Instant::now();
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Network".into(), detail: "Sending POST request...".into() });

    let response = client
        .post(&url)
        .header("x-api-key", api_key)
        .header("anthropic-version", api_version)
        .header("content-type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            let hint = if err_str.contains("dns") || err_str.contains("name") { "Hint: check network connection or DNS settings" }
            else if err_str.contains("refused") { "Hint: target server not responding" }
            else if err_str.contains("tls") { "Hint: SSL/TLS certificate issue" }
            else if err_str.contains("timeout") { "Hint: request timed out" }
            else { "Hint: check network/firewall/proxy" };
            logs.push(TestLogEntry { timestamp: now(), level: "ERROR".into(), phase: "Network".into(), detail: format!("Request failed: {} | {}", err_str, hint) });
            anyhow::anyhow!("Connection failed: {} ({})", err_str, hint)
        })?;

    let elapsed = start.elapsed();
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Network".into(), detail: format!("Response received in {}ms", elapsed.as_millis()) });
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("HTTP Status: {} {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or("Unknown")) });

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        logs.push(TestLogEntry { timestamp: now(), level: "ERROR".into(), phase: "Response".into(), detail: format!("Error body: {}", error_text) });
        anyhow::bail!("Connection failed (HTTP {}): {}", status, error_text);
    }

    let data: serde_json::Value = response.json().await?;
    logs.push(TestLogEntry { timestamp: now(), level: "DEBUG".into(), phase: "Response".into(), detail: format!("Response body: {}", serde_json::to_string_pretty(&data).unwrap_or_default()) });

    let model_resp = data.get("model").and_then(|m| m.as_str()).unwrap_or("unknown");
    let reply_text = data["content"].as_array().and_then(|blocks| {
        blocks.iter().find(|b| b["type"] == "text").and_then(|b| b["text"].as_str())
    }).unwrap_or("(no text content)");
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("Model confirmed: {}", model_resp) });
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("Reply text: {}", reply_text) });

    if let Some(u) = data.get("usage") {
        logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Usage".into(), detail: format!("Input tokens: {}, Output tokens: {}",
            u.get("input_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            u.get("output_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        )});
        if let Some(cr) = u.get("cache_read_input_tokens").and_then(|v| v.as_u64()) {
            logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Usage".into(), detail: format!("Cache read tokens: {}", cr) });
        }
    }

    Ok(data)
}

/// 共享的发送消息处理结果（消除 commands.rs 与 conversation_routes.rs 的 ~80% 代码重复）
pub struct SendMessageResult {
    pub reply_text: String,
    pub total_tokens: Option<i32>,
    pub metadata_str: Option<String>,
    pub usage: Option<UsageInfo>,
    pub tool_calls: Vec<ToolCallInfo>,
    pub tool_executions: Vec<ToolExecutionInfo>,
}

/// 从 ChatResponse 构建统一的 SendMessageResult（含 reply_text 格式化 + usage 提取 + 元数据构建）
pub fn build_send_message_result(response: ChatResponse, default_model: &str) -> SendMessageResult {
    let reply_text = if !response.text.is_empty() {
        response.text.clone()
    } else if !response.tool_calls.is_empty() {
        let mut parts = Vec::new();
        for tc in &response.tool_calls {
            parts.push(format!("[使用工具: {}]", tc.name));
            if let Ok(input_str) = serde_json::to_string(&tc.input) {
                if input_str.len() < 200 { parts.push(format!("  参数: {}", input_str)); }
            }
        }
        parts.join("\n")
    } else {
        "(无回复)".to_string()
    };

    let (total_tokens, metadata_str) = match &response.usage {
        Some(usage) => {
            let total = usage.input_tokens + usage.output_tokens;
            let meta = serde_json::json!({
                "input_tokens": usage.input_tokens,
                "output_tokens": usage.output_tokens,
                "cache_read": usage.cache_read_tokens.or(usage.cache_creation_tokens),
                "model": default_model
            });
            (Some(total as i32), Some(meta.to_string()))
        }
        _ => (None, None),
    };

    SendMessageResult { reply_text, total_tokens, metadata_str, usage: response.usage, tool_calls: response.tool_calls, tool_executions: response.tool_executions }
}

/// 测试OpenAI兼容API连接 — 记录详细的请求和响应诊断日志
async fn test_openai_connection_detailed(
    client: &reqwest::Client,
    base_url: &str,
    api_key: &str,
    model: &str,
    test_message: &str,
    logs: &mut Vec<TestLogEntry>,
) -> Result<serde_json::Value, anyhow::Error> {
    let now = || chrono::Local::now().format("%H:%M:%S%.3f").to_string();
    let base = base_url.trim_end_matches('/');
    let url = if base.ends_with("/chat/completions") { base.to_string() } else { format!("{}/chat/completions", base) };
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Request".into(), detail: format!("Target URL: {}", url) });

    let mut body = serde_json::json!({
        "model": model,
        "max_tokens": 64,
        "messages": [{"role": "user", "content": test_message}]
    });
    if !model_ignores_temperature(model) {
        body["temperature"] = serde_json::json!(effective_temperature(model, 0.0));
    }
    if model_uses_reasoning_effort(model) {
        body["reasoning_effort"] = serde_json::json!("medium");
    }
    if let Some(thinking) = build_thinking_param(model, 4096) {
        body["thinking"] = thinking;
    }
    logs.push(TestLogEntry { timestamp: now(), level: "DEBUG".into(), phase: "Request".into(), detail: format!("Request body: {}", serde_json::to_string_pretty(&body).unwrap_or_default()) });

    let start = std::time::Instant::now();
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Network".into(), detail: "Sending POST request...".into() });

    let response = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", api_key))
        .header("content-type", "application/json")
        .timeout(std::time::Duration::from_secs(30))
        .json(&body)
        .send()
        .await
        .map_err(|e| {
            let err_str = e.to_string();
            let hint = if err_str.contains("dns") || err_str.contains("name") { "Hint: check network connection or DNS settings" }
            else if err_str.contains("refused") { "Hint: target server not responding, check address" }
            else if err_str.contains("tls") || err_str.contains("certificate") { "Hint: SSL/TLS certificate verification failed" }
            else if err_str.contains("timeout") || err_str.contains("timed out") { "Hint: request timed out, server too slow" }
            else { "Hint: check network/firewall/proxy settings" };
            logs.push(TestLogEntry { timestamp: now(), level: "ERROR".into(), phase: "Network".into(), detail: format!("Request failed: {} | {}", err_str, hint) });
            anyhow::anyhow!("Connection failed: {} ({})", err_str, hint)
        })?;

    let elapsed = start.elapsed();
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Network".into(), detail: format!("Response received in {}ms", elapsed.as_millis()) });
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("HTTP Status: {} {}", response.status().as_u16(), response.status().canonical_reason().unwrap_or("Unknown")) });

    if !response.status().is_success() {
        let status = response.status();
        let error_text = response.text().await.unwrap_or_default();
        logs.push(TestLogEntry { timestamp: now(), level: "ERROR".into(), phase: "Response".into(), detail: format!("Error body: {}", error_text) });
        anyhow::bail!("Connection failed (HTTP {}): {}", status, error_text);
    }

    let data: serde_json::Value = response.json().await?;
    logs.push(TestLogEntry { timestamp: now(), level: "DEBUG".into(), phase: "Response".into(), detail: format!("Response body: {}", serde_json::to_string_pretty(&data).unwrap_or_default()) });

    let model_resp = data.get("model").and_then(|m| m.as_str()).unwrap_or(model);
    let reply_text = data["choices"].as_array().and_then(|a| a.first())
        .and_then(|c| c["message"]["content"].as_str())
        .unwrap_or("(no text content)");
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("Model confirmed: {}", model_resp) });
    logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Response".into(), detail: format!("Reply text: {}", reply_text) });

    if let Some(u) = data.get("usage") {
        logs.push(TestLogEntry { timestamp: now(), level: "INFO".into(), phase: "Usage".into(), detail: format!("Prompt tokens: {}, Completion tokens: {}",
            u.get("prompt_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
            u.get("completion_tokens").and_then(|v| v.as_u64()).unwrap_or(0),
        )});
    }

    Ok(data)
}
