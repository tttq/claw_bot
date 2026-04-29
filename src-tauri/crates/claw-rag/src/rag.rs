// Claw Desktop - RAG核心 - 记忆的存储、检索、实体提取、压缩逻辑
// 功能：文本向量化、多路融合检索、实体提取、记忆管理、压缩、用户画像
// 架构：增强记忆系统 → memory_units(核心) + entities(实体) + memory_links(关系) + FTS5全文索引
// 向量化：LocalEmbedder ONNX 384维 > Feature Hashing 128维 fallback

use claw_db::database::Database;
use claw_db::db::{get_db, try_get_agent_db};
use sea_orm::{EntityTrait, ColumnTrait, ActiveModelTrait, Set, QueryFilter, ConnectionTrait, QueryOrder, QuerySelect};
use uuid::Uuid;

const VECTOR_DIM: usize = crate::local_embedder::EMBEDDING_DIM;

/// 用户画像：从对话历史中提取的用户偏好、技术栈、沟通风格等长期特征
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct UserProfile {
    pub preferences: Vec<String>,
    pub frequent_topics: Vec<String>,
    pub coding_style: String,
    pub tech_stack: Vec<String>,
    pub communication_style: String,
    pub goals: Vec<String>,
    pub total_interactions: i64,
}

impl Default for UserProfile {
    fn default() -> Self {
        Self {
            preferences: Vec::new(),
            frequent_topics: Vec::new(),
            coding_style: "unknown".to_string(),
            tech_stack: Vec::new(),
            communication_style: "neutral".to_string(),
            goals: Vec::new(),
            total_interactions: 0,
        }
    }
}

/// 获取常用模型的Context Window大小（token数）
pub fn get_model_context_window(model_name: &str) -> i64 {
    let model = model_name.to_lowercase();
    if model.contains("claude-3-5") || model.contains("claude-3.5") { 200000 }
    else if model.contains("claude-3") || model.contains("sonnet-3") { 200000 }
    else if model.contains("claude-opus") { 200000 }
    else if model.contains("claude-sonnet-4") || model.contains("claude-4-sonnet") { 200000 }
    else if model.contains("gpt-4o") || model.contains("gpt-4-o") { 128000 }
    else if model.contains("gpt-4-turbo") { 128000 }
    else if model.contains("gpt-4") { 8192 }
    else if model.contains("gpt-3.5") || model.contains("gpt-35") { 16385 }
    else if model.contains("deepseek") { 128000 }
    else if model.contains("qwen") || model.contains("tongyi") { 131072 }
    else if model.contains("glm") || model.contains("zhipu") { 128000 }
    else if model.contains("llama") { 131072 }
    else { 100000 }
}

/// 计算对话压缩阈值 — 根据模型窗口大小和配置计算触发压缩的token数
pub fn calc_compaction_threshold(model_name: &str, config_override: Option<u64>) -> i64 {
    if let Some(override_val) = config_override {
        return override_val as i64;
    }
    let ctx = get_model_context_window(model_name);
    std::cmp::max((ctx as f64 * 0.7) as i64, 50000)
}

// ==================== 向量化核�?====================

/// 文本向量化 — 优先使用本地嵌入模型，降级使用特征哈希
pub fn embed_text(text: &str) -> Vec<f32> {
    crate::local_embedder::embed_text_fallback(text)
}

pub use claw_db::vector_store::{vector_to_bytes, bytes_to_vector, cosine_similarity};

// ==================== Agent System Prompt 集成 ====================

/// 获取Agent的系统提示词
pub async fn get_agent_system_prompt(agent_id: &str) -> Option<String> {
    let agent_db = try_get_agent_db()?;
    if let Ok(Some(agent)) = claw_db::db::agent_entities::agents::Entity::find_by_id(agent_id.to_string()).one(agent_db).await {
        let mut parts = Vec::new();
        if let Some(ref sp) = agent.system_prompt {
            parts.push(sp.clone());
        }
        if let Some(ref purpose) = agent.purpose {
            parts.push(format!("\n## Core Purpose\n{}", purpose));
        }
        if let Some(ref scope) = agent.scope {
            parts.push(format!("\n## Capability Scope\n{}", scope));
        }
        if parts.is_empty() { None } else { Some(parts.join("")) }
    } else {
        None
    }
}

/// 构建带Agent配置的系统提示词 — 包含Agent人设、工具目录、RAG上下文
pub async fn build_system_prompt_with_agent(config: &claw_config::config::AppConfig, agent_id: Option<&str>, max_turns: Option<usize>, tool_count: usize, tool_catalog: Option<&str>) -> String {
    let effective_max_turns = max_turns.unwrap_or(15);

    let custom_prompt = if let Some(aid) = agent_id {
        get_agent_system_prompt(aid).await.unwrap_or_default()
    } else {
        String::new()
    };

    let user_profile_ctx = if let Some(aid) = agent_id {
        get_user_profile_summary(aid).await.unwrap_or_default()
    } else {
        String::new()
    };

    let catalog_section = tool_catalog.unwrap_or("- Available tools: see `tools` parameter for full schemas");

    let base = if custom_prompt.is_empty() {
        r#"You are Claw, a capable AI assistant. You help users with coding, research, analysis, and creative tasks. Think step-by-step, use tools wisely, and communicate clearly. Proactively identify opportunities to help and anticipate needs."#.to_string()
    } else {
        custom_prompt
    };

    format!(r#"{base}

## Configuration
- Model: {model} ({provider}) | Max tool rounds: {max_turns} | Tools: {tool_count}

{user_profile_ctx}

{catalog_section}

## Response Signals — MANDATORY
You MUST end EVERY response with exactly ONE signal marker as the very last line. This is a system requirement, not optional.

Signal markers and when to use them:

| Signal | When to use | Example scenario |
|--------|------------|-----------------|
| `[RESPONSE_COMPLETE]` | You've fully answered, no further action needed | Answered a question, completed a task |
| `[INPUT_REQUIRED]` | You need user input/choice before continuing | Asking which option, need clarification |
| `[CONFIRM_REQUIRED]` | You need explicit confirmation for risky actions | About to delete files, run destructive commands |
| `[TASK_IN_PROGRESS]` | Multi-step task in progress, more steps coming | Working through a checklist, debugging |

Rules:
1. The signal MUST be on its own line as the LAST line of your response
2. If you omit a signal, `[RESPONSE_COMPLETE]` is assumed by the system
3. NEVER put text after the signal marker
4. For risky operations (file deletion, system changes, irreversible actions), ALWAYS use `[CONFIRM_REQUIRED]`
5. When asking the user a question that blocks progress, use `[INPUT_REQUIRED]`

Examples:
✅ Correct:
```
I've created the file `config.json` with your settings.

[RESPONSE_COMPLETE]
```

✅ Correct:
```
I found 3 large files. Which ones should I delete?
1. `node_modules/` (500MB)
2. `dist/` (200MB)  
3. `cache/` (150MB)

[INPUT_REQUIRED]
```

❌ Wrong — no signal:
```
I've created the file for you.
```

❌ Wrong — text after signal:
```
[RESPONSE_COMPLETE]
Let me know if you need anything else.
```

## Core Principles

**Reasoning First**: Think before acting. For general knowledge, just answer directly. Use tools only when you need real-time data, file access, or actions beyond text generation.

**Tool Intelligence**: Pick the right tool for the job. Plan multi-step sequences before executing. If a tool fails, analyze the error and try an alternative. Use `ToolSearch` when uncertain about available tools.

**Proactive & Helpful**: Anticipate needs and suggest improvements. Respect the user's direction. For recurring patterns, suggest creating a tool or skill.

**Task Management**: Break complex tasks into clear steps. Use `TodoWrite` for tracking. Stay focused — complete the current task before starting tangential work. Summarize when done.

## Memory & Context
- **Historical Reference** sections are SUMMARIZED memories from past conversations — treat as background knowledge, not current context
- **User Profile** sections contain learned preferences — use them to personalize responses naturally
- Don't force references to historical memories unless directly relevant

## Dynamic Extensions
- The Tool Catalog reflects ALL currently available tools, including dynamically loaded skills and MCP tools
- New tools may appear at runtime — check the catalog before assuming unavailability
- For recurring patterns, use `CreateTool` or `CreateSkill` to create dedicated tools

## Multi-Agent Collaboration
When the user mentions @AgentName, you are the main agent:

1. **Decompose**: Break the request into subtasks matching each agent's expertise
2. **Delegate**: Use the `Agent` tool with clear, specific prompts and context
3. **Monitor**: If a sub-agent returns generic/short output, re-prompt with more specific instructions
4. **Aggregate**: Synthesize results, resolve conflicts, attribute contributions. Fill gaps if any sub-agent failed

Use `fork` mode for independent parallel tasks, `background` for fire-and-forget tasks.

## Browser Workflow
When explicitly asked to visit/analyze a webpage:
1. `browser_launch` → 2. `browser_navigate` → 3. `browser_get_content` → 4. Analyze & summarize
Fallback: Use `web_fetch` for simple URL fetching. Do NOT proactively browse unless asked.

## Desktop Automation
When asked to interact with desktop applications:

**Complex tasks** → Delegate to `desktop-agent` via the `Agent` tool (recommended):
```
Agent tool: prompt="Open Chrome", mode="fork"
```

**Simple single-step actions** → Use tools directly:
- Open app: `KeyboardPress("Super")` → `KeyboardType("app name")` → `KeyboardPress("Enter")`
- Read screen: `CaptureScreen` (OCR text summary) or `OcrRecognizeScreen` (detailed with coordinates)
- Click element: `OcrRecognizeScreen` → find coordinates → `MouseClick(x, y)` or `MouseDoubleClick(x, y)`
- Quick automation: `ExecuteAutomation` with a natural-language instruction

**Safety rules**: Always capture screen before clicking. Verify after each action. If automation fails after 2 attempts, stop and explain. Never automate destructive actions without `[CONFIRM_REQUIRED]`.

## Response Style
- Be natural and conversational — write like a knowledgeable colleague
- Use Markdown well: headers for structure, lists for enumeration, code blocks with language tags
- Be concise but complete — don't pad responses, don't omit important details
- End with the correct signal marker"#,
        base = base,
        model = config.model.default_model,
        provider = config.model.provider,
        max_turns = effective_max_turns,
        tool_count = tool_count,
        user_profile_ctx = user_profile_ctx,
        catalog_section = catalog_section,
    )
}

// ==================== 用户画像系统（按Agent隔离�?===================

/// 获取用户画像
pub async fn get_user_profile(agent_id: &str) -> Result<UserProfile, String> {
    let agent_db = try_get_agent_db().ok_or("Agent DB not initialized")?;
    
    match claw_db::db::agent_entities::agent_profiles::Entity::find_by_id(agent_id.to_string())
        .one(agent_db).await
    {
        Ok(Some(profile)) => {
            match serde_json::from_str::<UserProfile>(&profile.profile_json) {
                Ok(p) => Ok(p),
                Err(_) => Ok(UserProfile::default()),
            }
        }
        _ => Ok(UserProfile::default()),
    }
}

/// 获取用户画像摘要文本
pub async fn get_user_profile_summary(agent_id: &str) -> Result<String, String> {
    let profile = get_user_profile(agent_id).await?;
    if profile.total_interactions == 0 {
        return Ok(String::new());
    }

    let mut summary = String::from("\n\n--- User Profile (Learned Preferences) ---\n");
    if !profile.preferences.is_empty() {
        summary.push_str(&format!("- Preferences: {}\n", profile.preferences.join(", ")));
    }
    if !profile.frequent_topics.is_empty() {
        summary.push_str(&format!("- Frequent topics: {}\n", profile.frequent_topics.join(", ")));
    }
    if profile.coding_style != "unknown" {
        summary.push_str(&format!("- Coding style: {}\n", profile.coding_style));
    }
    if !profile.tech_stack.is_empty() {
        summary.push_str(&format!("- Tech stack: {}\n", profile.tech_stack.join(", ")));
    }
    if !profile.goals.is_empty() {
        summary.push_str(&format!("- Goals: {}\n", profile.goals.join(", ")));
    }
    summary.push_str(&format!("- Total interactions: {}\n", profile.total_interactions));
    summary.push_str("--- End User Profile ---\n");
    Ok(summary)
}

/// 更新用户画像 — 从对话中提取偏好并更新数据库
pub async fn update_user_profile(agent_id: &str, user_message: &str, assistant_response: &str) -> Result<(), String> {
    let agent_db = try_get_agent_db().ok_or("Agent DB not initialized")?;
    let mut profile = get_user_profile(agent_id).await?;
    profile.total_interactions += 1;

    let combined = format!("User: {}\nAssistant: {}", user_message, assistant_response);
    let lower = combined.to_lowercase();

    let preference_indicators = [
        ("prefer", "prefer"), ("like", "like"), ("always", "always"),
        ("never", "never"), ("should", "should"), ("must", "must"),
        ("don't", "avoid"), ("avoid", "avoid"), ("hate", "dislike"),
        ("love", "love"), ("favorite", "favorite"), ("best", "best"),
        ("喜欢", "偏好"), ("偏好", "偏好"), ("习惯", "习惯"),
        ("不喜欢", "不喜欢"), ("讨厌", "讨厌"), ("最", "最"),
        ("总是", "总是"), ("从不", "从不"), ("应该", "应该"),
    ];

    use std::collections::HashSet;
    let mut pref_seen: HashSet<String> = HashSet::new();
    for (keyword, category) in &preference_indicators {
        if lower.contains(keyword) {
            if let Some(pos) = lower.find(keyword) {
                let start = pos.saturating_sub(10);
                let end = (pos + keyword.len() + 80).min(combined.len());
                let snippet = combined[start..end].replace('\n', " ").trim().to_string();
                let dedup_key = claw_types::truncate_str_safe(&snippet, 30).to_string();
                if !snippet.is_empty() && !pref_seen.contains(&dedup_key) {
                    pref_seen.insert(dedup_key.clone());
                    let tagged = format!("[{}] {}", category, snippet);
                    profile.preferences.push(tagged);
                    if profile.preferences.len() > 20 { profile.preferences.remove(0); }
                }
            }
        }
    }

    let tech_keywords = [
        "rust", "python", "typescript", "javascript", "react", "vue", "angular", "svelte",
        "nextjs", "nuxt", "sveltekit", "express", "fastapi", "django", "flask", "spring",
        "go", "java", "c++", "c#", "swift", "kotlin", "dart", "ruby", "php",
        "tauri", "electron", "flutter", "react native",
        "postgresql", "mysql", "sqlite", "redis", "mongodb", "elasticsearch",
        "docker", "kubernetes", "terraform", "ansible", "nginx",
        "aws", "azure", "gcp", "vercel", "netlify",
        "git", "github", "gitlab", "vs code", "neovim",
        "api", "rest", "graphql", "grpc", "websocket",
        "database", "frontend", "backend", "devops", "testing", "algorithm",
        "machine learning", "ai", "llm", "openai", "anthropic",
    ];

    let mut topic_seen: HashSet<&str> = HashSet::new();
    for topic in &tech_keywords {
        if lower.contains(topic) && !topic_seen.contains(*topic) {
            topic_seen.insert(topic);
            profile.frequent_topics.push(topic.to_string());
            if profile.frequent_topics.len() > 15 { profile.frequent_topics.remove(0); }
        }
    }

    if lower.contains("functional") || lower.contains("declarative") || lower.contains("函数式") {
        profile.coding_style = "functional/declarative".to_string();
    } else if lower.contains("object-oriented") || lower.contains("oop") || lower.contains("面向对象") {
        profile.coding_style = "OOP".to_string();
    } else if lower.contains("procedural") || lower.contains("过程式") {
        profile.coding_style = "procedural".to_string();
    }

    let tech_stack_keywords = [
        "react", "vue", "angular", "svelte", "nextjs", "nuxt",
        "express", "fastapi", "django", "spring", "flask",
        "rust", "go", "node", "nodejs",
        "postgresql", "mysql", "redis", "mongodb", "sqlite",
        "kubernetes", "docker", "terraform",
        "tauri", "electron",
    ];

    let tech_stack_set: HashSet<String> = profile.tech_stack.iter().map(|t| t.to_lowercase()).collect();
    for tech in &tech_stack_keywords {
        if lower.contains(tech) && !tech_stack_set.contains(*tech) {
            profile.tech_stack.push(tech.to_string());
            if profile.tech_stack.len() > 12 { profile.tech_stack.remove(0); }
        }
    }

    let goal_indicators = [
        ("i want to ", "goal"), ("i need to ", "goal"), ("i'm trying to ", "goal"),
        ("my goal is ", "goal"), ("i'm working on ", "goal"),
        ("我想", "目标"), ("我需要", "目标"), ("我正在", "目标"), ("我的目标是", "目标"),
    ];

    let goal_keys: HashSet<String> = profile.goals.iter()
        .filter_map(|g| Some(claw_types::truncate_str_safe(g, 20).to_string()))
        .collect();
    for (indicator, _tag) in &goal_indicators {
        if lower.contains(indicator) {
            if let Some(pos) = lower.find(indicator) {
                let end = (pos + indicator.len() + 60).min(combined.len());
                let goal_text = combined[pos..end].replace('\n', " ").trim().to_string();
                let goal_key = claw_types::truncate_str_safe(&goal_text, 20).to_string();
                if !goal_text.is_empty() && !goal_keys.contains(&goal_key) {
                    profile.goals.push(goal_text);
                    if profile.goals.len() > 8 { profile.goals.remove(0); }
                }
            }
        }
    }

    if combined.len() > 500 { profile.communication_style = "detailed".to_string(); }
    else if combined.len() < 100 && profile.communication_style != "detailed" { profile.communication_style = "concise".to_string(); }

    let profile_json = serde_json::to_string(&profile).unwrap_or_default();
    let now = chrono::Utc::now().timestamp();

    let existing = claw_db::db::agent_entities::agent_profiles::Entity::find_by_id(agent_id.to_string())
        .one(agent_db).await.map_err(|e| e.to_string())?;

    match existing {
        Some(_) => {
            let am = claw_db::db::agent_entities::agent_profiles::ActiveModel {
                agent_id: Set(agent_id.to_string()),
                profile_json: Set(profile_json),
                interaction_count: Set(profile.total_interactions),
                last_updated_at: Set(now),
                ..Default::default()
            };
            am.update(agent_db).await.map_err(|e| e.to_string())?;
        }
        None => {
            let am = claw_db::db::agent_entities::agent_profiles::ActiveModel {
                agent_id: Set(agent_id.to_string()),
                profile_json: Set(profile_json),
                interaction_count: Set(1),
                last_updated_at: Set(now),
                created_at: Set(now),
            };
            am.insert(agent_db).await.map_err(|e| e.to_string())?;
        }
    }

    log::info!("[Profile] Updated profile for agent {} (interactions: {})", claw_types::truncate_str_safe(agent_id, 8), profile.total_interactions);
    Ok(())
}

// ==================== 自适应压缩 ====================

/// 压缩对话历史 — 当token数超过阈值时自动触发RAG压缩
pub async fn compact_conversation_if_needed(
    conversation_id: &str,
    agent_id: Option<&str>,
    model_name: &str,
    config_threshold: Option<u64>,
) -> Result<bool, String> {
    let msgs = Database::get_messages(conversation_id).await.map_err(|e| e.to_string())?;
    let total_tokens: i64 = msgs.iter().map(|m| m.token_count.unwrap_or(0) as i64).sum();
    let threshold = calc_compaction_threshold(model_name, config_threshold);

    if total_tokens < threshold {
        return Ok(false);
    }

    let conv_preview: String = conversation_id.chars().take(12).collect();
    log::info!("[RAG:Compaction] Conv {} tokens={} >= threshold={}, model={}",
        conv_preview, total_tokens, threshold, model_name);

    let keep_recent = std::cmp::max(msgs.len() / 3, 2);
    let to_compact: Vec<_> = msgs.iter().take(msgs.len() - keep_recent).collect();
    if to_compact.is_empty() { return Ok(false); }

    let effective_agent_id = match agent_id {
        Some(aid) if !aid.is_empty() => aid.to_string(),
        _ => {
            log::info!("[RAG:Compaction] No agent_id for conv={}, using 'default' for compaction storage", claw_types::truncate_str_safe(conversation_id, 16));
            "default".to_string()
        }
    };

    let mut old_content = String::new();
    for m in &to_compact {
        old_content.push_str(&format!("[{}]: {}\n", m.role, m.content));
    }

    store_enhanced_memory(&effective_agent_id, Some(conversation_id), &old_content, "experience", "compaction", None, None).await.ok();

    let db = get_db().await;
    for m in &to_compact {
        let _ = claw_db::db::entities::messages::Entity::delete_by_id(&m.id).exec(db).await;
    }

    let summary_msg = format!(
        "[System: Compacted {} old messages into RAG (tokens was {}, now under {}). Use RAG context to retrieve details.]",
        to_compact.len(), total_tokens, threshold
    );
    Database::add_message(conversation_id, "system", &summary_msg, None, Some((summary_msg.len() / 4) as i32), Some(r#""source":"compaction""#.to_string())).await.map_err(|e| e.to_string())?;

    let conv_preview2: String = conversation_id.chars().take(12).collect();
    log::info!("[RAG:Compaction] Compacted {} messages from conv {}", to_compact.len(), conv_preview2);
    Ok(true)
}

/// 删除指定对话的所有记忆
pub async fn delete_conversation_memories(conversation_id: &str) -> Result<u64, String> {
    let db = get_db().await;
    let res = claw_db::db::entities::memory_units::Entity::delete_many()
        .filter(claw_db::db::entities::memory_units::Column::ConversationId.eq(conversation_id))
        .exec(db).await.map_err(|e: sea_orm::DbErr| e.to_string())?;
    let conv_preview: String = conversation_id.chars().take(12).collect();
    log::info!("[RAG:v2] Deleted {} memory units for conv {}", res.rows_affected, conv_preview);
    Ok(res.rows_affected)
}

/// 构建 RAG 增强的上下文注入字符串
/// Hindsight-inspired: 严格区分"当前会话上下文"和"历史参考记忆"
/// 关键修复：不再将历史对话的原始内容注入，只注入结构化摘要
/// 构建RAG上下文 — 混合检索（向量+BM25+时间）并融合排序
pub async fn build_rag_context(agent_id: Option<&str>, conversation_id: &str, user_query: &str) -> Result<String, String> {
    const MAX_RAG_CHARS: usize = 6000;

    let effective_agent_id = match agent_id {
        Some(aid) if !aid.is_empty() => aid.to_string(),
        _ => {
            log::info!("[RAG] No agent_id provided for conv={}, using 'default' global context", claw_types::truncate_str_safe(conversation_id, 16));
            "default".to_string()
        }
    };

    let mut ctx_parts: Vec<String> = Vec::new();

    let user_profile = get_user_profile_summary(&effective_agent_id).await.unwrap_or_default();
    if !user_profile.is_empty() {
        ctx_parts.push(user_profile);
    }

    match hybrid_retrieve(user_query, &effective_agent_id, Some(conversation_id), 5).await {
        Ok(memories) if !memories.is_empty() => {
            let mut history_section = String::from("\n\n--- Historical Reference (from past conversations) ---\n");
            history_section.push_str("NOTE: These are SUMMARIZED memories from PREVIOUS conversations, NOT part of the current conversation.\n");
            history_section.push_str("Use them as background knowledge only. Do NOT treat them as current conversation context.\n\n");

            for (i, mem) in memories.iter().enumerate() {
                if history_section.len() >= MAX_RAG_CHARS { break; }
                let source_conv = mem.source_type.clone();
                let fact_label = match mem.fact_type.as_str() {
                    "world" => "World Fact",
                    "experience" => "Past Experience",
                    "mental_model" => "Learned Pattern",
                    _ => &mem.fact_type,
                };
                let layer_label = mem.metadata.as_deref()
                    .and_then(|m| serde_json::from_str::<std::collections::HashMap<String, serde_json::Value>>(m).ok())
                    .and_then(|m| m.get("memory_layer").and_then(|v| v.as_str()).map(|s| s.to_string()))
                    .unwrap_or_else(|| "unknown".to_string());
                let display_label = match layer_label.as_str() {
                    "working" => "Working Memory",
                    "episodic" => "Episodic Memory",
                    "semantic" => "Semantic Memory",
                    "procedural" => "Procedural Memory",
                    _ => fact_label,
                };
                let time_ago = mem.occurred_at
                    .map(|t| {
                        let days = (chrono::Utc::now().timestamp() - t) / 86400;
                        if days == 0 { "today".to_string() }
                        else if days == 1 { "yesterday".to_string() }
                        else if days < 30 { format!("{} days ago", days) }
                        else if days < 365 { format!("{} months ago", days / 30) }
                        else { format!("{} years ago", days / 365) }
                    })
                    .unwrap_or_else(|| "unknown time".to_string());

                let preview = if mem.text.len() > 250 {
                    let safe_end = mem.text.char_indices().take(250).last().map(|(i, _)| i).unwrap_or(0);
                    format!("{}...", &mem.text[..safe_end])
                } else {
                    mem.text.clone()
                };
                history_section.push_str(&format!(
                    "{}. [{}] ({}) - {}\n   {}\n",
                    i + 1, display_label, time_ago, source_conv, preview
                ));
            }
            history_section.push_str("--- End Historical Reference ---\n");
            ctx_parts.push(history_section);

            log::info!("[RAG] Built context with {} historical memories for query '{}'",
                memories.len(), claw_types::truncate_str_safe(user_query, 40));
        }
        _ => {
            log::debug!("[RAG] No relevant memories for query '{}'", claw_types::truncate_str_safe(user_query, 40));
        }
    }

    if ctx_parts.is_empty() {
        Ok(String::new())
    } else {
        Ok(ctx_parts.join(""))
    }
}

// ==================== 增强记忆系统 v2 (Hindsight-inspired) ====================

/// 增强版记忆单元（含多维度元数据 + 四层记忆架构）
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct EnhancedMemoryUnit {
    pub id: String,
    pub text: String,
    pub fact_type: String,
    pub context: Option<String>,
    pub occurred_at: Option<i64>,
    pub source_type: String,
    pub tags: Option<String>,
    pub importance_score: f64,
    pub semantic_score: f64,
    pub bm25_score: f64,
    pub temporal_score: f64,
    pub final_score: f64,
    pub metadata: Option<String>,
}

/// 存储增强记忆 — 含向量化、实体提取、重要性评分、分层存储
pub async fn store_enhanced_memory(
    agent_id: &str,
    conversation_id: Option<&str>,
    text: &str,
    fact_type: &str,
    source_type: &str,
    context: Option<&str>,
    tags: Option<&str>,
) -> Result<String, String> {
    let db = get_db().await;
    let now = chrono::Utc::now().timestamp();
    let id = Uuid::new_v4().to_string();

    let vector = embed_text(text);
    let embedding_bytes = vector_to_bytes(&vector);

    let importance = calc_importance_score(fact_type, source_type, tags, text.len());

    let memory_layer = crate::memory_layers::classify_to_layer(fact_type, source_type, tags, importance);
    let layer_config = crate::memory_layers::LayerConfig::for_layer(memory_layer);
    let expires_at = layer_config.ttl_seconds.map(|ttl| now + ttl);

    let mut metadata_map = std::collections::HashMap::new();
    metadata_map.insert("memory_layer".to_string(), serde_json::json!(memory_layer.as_str()));
    if let Some(ea) = expires_at {
        metadata_map.insert("expires_at".to_string(), serde_json::json!(ea));
    }
    let metadata_json = serde_json::to_string(&metadata_map).unwrap_or_default();

    let am = claw_db::db::entities::memory_units::ActiveModel {
        id: Set(id.clone()),
        agent_id: Set(agent_id.to_string()),
        conversation_id: Set(conversation_id.map(|s| s.to_string())),
        text: Set(text.to_string()),
        embedding: Set(embedding_bytes.clone()),
        fact_type: Set(fact_type.to_string()),
        context: Set(context.map(|s| s.to_string())),
        occurred_at: Set(Some(now)),
        mentioned_at: Set(Some(now)),
        source_type: Set(source_type.to_string()),
        tags: Set(tags.map(|s| s.to_string())),
        importance_score: Set(importance),
        access_count: Set(0),
        memory_layer: Set(Some(memory_layer.as_str().to_string())),
        expires_at: Set(expires_at),
        metadata: Set(Some(metadata_json)),
        created_at: Set(now),
        updated_at: Set(now),
        ..Default::default()
    };

    am.insert(db).await.map_err(|e| e.to_string())?;

    // 同步写入 sqlite-vec 向量虚拟表（加速语义检索）
    if let Err(e) = db.execute(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT OR REPLACE INTO memory_vectors(rowid, embedding, memory_unit_id, agent_id) \
         VALUES((SELECT rowid FROM memory_units WHERE id = ?1), ?2, ?1, ?3)",
        [id.clone().into(), embedding_bytes.into(), agent_id.to_string().into()],
    )).await {
        log::warn!("[RAG:v2] sqlite-vec insert failed (using BLOB fallback): {}", e);
    }

    // 同步更新 FTS5 索引
    if let Err(e) = db.execute(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "INSERT INTO memory_units_fts(rowid, text) VALUES(?1, ?2)",
        [id.clone().into(), text.into()],
    )).await {
        log::warn!("[RAG:v2] FTS5 index update failed: {}", e);
    }

    // 自动提取实体并关�?
    if let Err(e) = extract_and_link_entities(&id, agent_id, text).await {
        log::warn!("[RAG:v2] Entity extraction failed: {}", e);
    }

    let id_preview: String = id.chars().take(8).collect();
    let agent_preview: String = agent_id.chars().take(8).collect();
    log::info!("[RAG:v2] Stored enhanced memory unit {} for agent {}", id_preview, agent_preview);
    Ok(id)
}

/// BM25关键词搜索 — 基于词频的全文检索
async fn bm25_search(
    query: &str,
    agent_id: &str,
    limit: usize,
) -> Vec<(String, f64)> {
    let db = get_db().await;
    let mut results = Vec::new();

    let query_clean: String = query.chars()
        .filter(|c| c.is_alphanumeric() || *c == ' ')
        .collect::<String>()
        .to_lowercase()
        .split_whitespace()
        .take(10)
        .collect::<Vec<_>>()
        .join(" ");

    if query_clean.is_empty() { return results; }

    match db.query_all(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT mu.id, rank FROM memory_units_fts fts \
         JOIN memory_units mu ON mu.id = fts.rowid \
         WHERE memory_units_fts MATCH ?1 AND mu.agent_id = ?2 \
         ORDER BY rank LIMIT ?3",
        [query_clean.into(), agent_id.into(), (limit as i64).into()],
    )).await {
        Ok(rows) => for row in rows {
            if let Some(id) = row.try_get::<String>("", "id").ok()
            { if let Some(rank) = row.try_get::<f64>("", "rank").ok() {
                let score = 1.0 / (1.0 + rank as f64);
                results.push((id, score));
            }}
        }
        Err(e) => log::warn!("[RAG:v2] BM25 search error: {}", e),
    }
    results
}

/// 时间衰减搜索 — 优先返回近期记忆
async fn temporal_search(
    agent_id: &str,
    limit: usize,
) -> Vec<(String, f64)> {
    let db = get_db().await;
    let now = chrono::Utc::now().timestamp();
    let mut results = Vec::new();

    const HALF_LIFE_DAYS: f64 = 30.0;

    match claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::AgentId.eq(agent_id))
        .order_by_desc(claw_db::db::entities::memory_units::Column::OccurredAt)
        .limit(limit as u64)
        .all(db).await
    {
        Ok(units) => for unit in units {
            if let Some(occurred_at) = unit.occurred_at {
                let days_elapsed = ((now - occurred_at) as f64) / 86400.0;
                let decay = 2.0_f64.powf(-days_elapsed / HALF_LIFE_DAYS);
                let importance_boost = unit.importance_score.min(3.0);
                let temporal_score = decay * importance_boost;
                results.push((unit.id.clone(), temporal_score));
            }
        }
        Err(e) => log::warn!("[RAG:v2] Temporal search error: {}", e),
    }
    results
}

/// RRF融合排序 — 将多路检索结果通过倒数排名融合为统一排序
fn rrf_fusion(
    semantic_results: &[(String, f64)],
    bm25_results: &[(String, f64)],
    temporal_results: &[(String, f64)],
    k: f64,
    limit: usize,
) -> Vec<(String, f64)> {
    use std::collections::HashMap;
    let mut scores: HashMap<String, f64> = HashMap::new();

    for (idx, (id, _score)) in semantic_results.iter().enumerate() {
        *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (k + idx as f64 + 1.0);
    }

    for (idx, (id, _score)) in bm25_results.iter().enumerate() {
        *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (k + idx as f64 + 1.0);
    }

    for (idx, (id, _score)) in temporal_results.iter().enumerate() {
        *scores.entry(id.clone()).or_insert(0.0) += 1.0 / (k + idx as f64 + 1.0);
    }

    let mut ranked: Vec<(String, f64)> = scores.into_iter().collect();
    ranked.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    ranked.into_iter().take(limit).collect()
}

/// 混合检索 — 向量相似度 + BM25 + 时间衰减，RRF融合排序
pub async fn hybrid_retrieve(
    query: &str,
    agent_id: &str,
    conversation_id: Option<&str>,
    limit: usize,
) -> Result<Vec<EnhancedMemoryUnit>, String> {
    let query_vec = embed_text(query);

    let db = get_db().await;

    let base_filter = claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::AgentId.eq(agent_id));

    let filter_with_conv = if let Some(cid) = conversation_id {
        base_filter.filter(
            sea_orm::Condition::any()
                .add(claw_db::db::entities::memory_units::Column::ConversationId.is_null())
                .add(claw_db::db::entities::memory_units::Column::ConversationId.ne(cid))
        )
    } else {
        base_filter
    };

    let all_units = filter_with_conv
        .order_by_desc(claw_db::db::entities::memory_units::Column::CreatedAt)
        .limit(Some((limit * 10) as u64))
        .all(db).await.map_err(|e| e.to_string())?;

    let mut semantic_results: Vec<(String, f64)> = Vec::new();

    // 优先使用 sqlite-vec 向量虚拟表进行高效语义检索
    let query_bytes = vector_to_bytes(&query_vec);
    match db.query_all(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "SELECT mv.memory_unit_id, \
         1.0 - (vector_distance_cosine(mv.embedding, ?1)) AS similarity \
         FROM memory_vectors mv \
         WHERE mv.agent_id = ?2 \
         AND 1.0 - vector_distance_cosine(mv.embedding, ?1) > 0.25 \
         ORDER BY similarity DESC LIMIT ?3",
        [query_bytes.into(), agent_id.to_string().into(), ((limit * 3) as i64).into()],
    )).await {
        Ok(rows) => for row in rows {
            if let Some(unit_id) = row.try_get::<String>("", "memory_unit_id").ok() {
                if let Some(sim) = row.try_get::<f64>("", "similarity").ok() {
                    semantic_results.push((unit_id, sim));
                }
            }
        },
        Err(e) => {
            log::debug!("[RAG] sqlite-vec search unavailable, using BLOB fallback: {}", e);
            // 降级方案：BLOB 手动余弦相似度（限制扫描数量防止 O(n) 全量计算）
            let scan_limit = (limit * 5).min(all_units.len());
            for unit in all_units.iter().take(scan_limit) {
                let stored_vec = bytes_to_vector(&unit.embedding);
                if stored_vec.len() == VECTOR_DIM {
                    let sim = cosine_similarity(&stored_vec, &query_vec);
                    if sim > 0.25 {
                        semantic_results.push((unit.id.clone(), sim));
                    }
                }
            }
        }
    }

    semantic_results.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
    semantic_results.truncate(limit * 3);

    let bm25_results = bm25_search(query, agent_id, limit * 3).await;
    let temporal_results = temporal_search(agent_id, limit * 3).await;

    let fused = rrf_fusion(&semantic_results, &bm25_results, &temporal_results, 60.0, limit);

    let mut final_results = Vec::new();
    let semantic_map: std::collections::HashMap<String, f64> =
        semantic_results.into_iter().collect();
    let bm25_map: std::collections::HashMap<String, f64> =
        bm25_results.into_iter().collect();
    let temporal_map: std::collections::HashMap<String, f64> =
        temporal_results.into_iter().collect();
    let units_map: std::collections::HashMap<String, &claw_db::db::entities::memory_units::Model> =
        all_units.iter().map(|u| (u.id.clone(), u)).collect();

    for (unit_id, rrf_score) in fused {
        if let Some(unit) = units_map.get(&unit_id) {
            let result = EnhancedMemoryUnit {
                id: unit.id.clone(),
                text: unit.text.clone(),
                fact_type: unit.fact_type.clone(),
                context: unit.context.clone(),
                occurred_at: unit.occurred_at,
                source_type: unit.source_type.clone(),
                tags: unit.tags.clone(),
                importance_score: unit.importance_score,
                semantic_score: *semantic_map.get(&unit_id).unwrap_or(&0.0),
                bm25_score: *bm25_map.get(&unit_id).unwrap_or(&0.0),
                temporal_score: *temporal_map.get(&unit_id).unwrap_or(&0.0),
                final_score: rrf_score,
                metadata: unit.metadata.clone(),
            };
            final_results.push(result);
        }
    }

    log::info!("[RAG] Hybrid retrieve: query='{}' →{} results (excluded current conv)",
        claw_types::truncate_str_safe(query, 40), final_results.len());
    Ok(final_results)
}

/// 实体提取与关联 — 从文本中识别命名实体并建立共现关系
async fn extract_and_link_entities(
    memory_unit_id: &str,
    agent_id: &str,
    text: &str,
) -> Result<(), String> {
    let tech_keywords = [
        ("Rust", "technology"), ("Python", "technology"), ("TypeScript", "technology"),
        ("JavaScript", "technology"), ("React", "technology"), ("Vue", "technology"),
        ("PostgreSQL", "technology"), ("SQLite", "technology"), ("Redis", "technology"),
        ("Docker", "technology"), ("Kubernetes", "technology"), ("Linux", "technology"),
        ("Git", "technology"), ("API", "concept"), ("REST", "concept"),
        ("GraphQL", "concept"), ("Tauri", "technology"), ("Sea-ORM", "technology"),
        ("SQL", "concept"), ("HTML", "technology"), ("CSS", "technology"),

        ("Go", "technology"), ("Java", "technology"), ("C++", "technology"),
        ("Node.js", "technology"), ("Next.js", "technology"), ("Angular", "technology"),
        ("Svelte", "technology"), ("SolidJS", "technology"), ("Flutter", "technology"),
        ("Dart", "technology"), ("Swift", "technology"), ("Kotlin", "technology"),
        ("MongoDB", "technology"), ("MySQL", "technology"), ("Elasticsearch", "technology"),
        ("AWS", "technology"), ("Azure", "technology"), ("GCP", "technology"),
        ("Terraform", "technology"), ("Ansible", "technology"), ("Nginx", "technology"),
        ("Webpack", "technology"), ("Vite", "technology"), ("ESLint", "technology"),
        ("Jest", "concept"), ("Cypress", "technology"), ("Playwright", "technology"),
        ("CI/CD", "concept"), ("DevOps", "concept"), ("Microservices", "concept"),
        ("Serverless", "concept"), ("WebSocket", "concept"), ("gRPC", "concept"),
        ("OAuth", "concept"), ("JWT", "concept"), ("HTTPS", "concept"),
        ("Machine Learning", "concept"), ("AI", "concept"), ("LLM", "concept"),
        ("GPT", "concept"), ("Claude", "concept"), ("OpenAI", "organization"),
        ("Anthropic", "organization"), ("Google", "organization"),
        ("Microsoft", "organization"), ("Apple", "organization"),
        ("GitHub", "technology"), ("GitLab", "technology"), ("VS Code", "technology"),
        ("Neovim", "technology"), ("Vim", "technology"), ("Emacs", "technology"),
    ];

    let db = get_db().await;
    let now = chrono::Utc::now().timestamp();

    for (keyword, entity_type) in &tech_keywords {
        if text.contains(keyword) {
            let entity_id = format!("{}_{}", agent_id, keyword.to_lowercase());

            match claw_db::db::entities::entities::Entity::find_by_id(entity_id.clone())
                .one(db).await
            {
                Ok(Some(existing)) => {
                    let mut am: claw_db::db::entities::entities::ActiveModel = existing.into();
                    let current_count = match am.mention_count {
                        sea_orm::ActiveValue::Set(v) => v,
                        sea_orm::ActiveValue::Unchanged(v) => v,
                        sea_orm::ActiveValue::NotSet => 0,
                    };
                    am.mention_count = Set(current_count + 1);
                    am.last_seen = Set(now);
                    am.update(db).await.ok();
                }
                Ok(None) => {
                    let am = claw_db::db::entities::entities::ActiveModel {
                        id: Set(entity_id.clone()),
                        agent_id: Set(agent_id.to_string()),
                        canonical_name: Set(keyword.to_string()),
                        entity_type: Set(entity_type.to_string()),
                        first_seen: Set(now),
                        last_seen: Set(now),
                        mention_count: Set(1),
                        ..Default::default()
                    };
                    am.insert(db).await.ok();
                }
                _ => {}
            }

            let link_am = claw_db::db::entities::unit_entities::ActiveModel {
                unit_id: Set(memory_unit_id.to_string()),
                entity_id: Set(entity_id),
                role: Set(Some("mentioned".to_string())),
                ..Default::default()
            };
            link_am.insert(db).await.ok();
        }
    }

    Ok(())
}

// ==================== 工具/Skill 记忆分级系统 ====================

const SYSTEM_AGENT_ID: &str = "__system__";
const TOOL_MEMORY_TAG: &str = "tool_knowledge";
const SKILL_MEMORY_TAG: &str = "skill_knowledge";

pub const MEMORY_TIER_CORE: &str = "core";
pub const MEMORY_TIER_IMPORTANT: &str = "important";
pub const MEMORY_TIER_NORMAL: &str = "normal";
pub const MEMORY_TIER_EPHEMERAL: &str = "ephemeral";

const MAX_MEMORY_UNITS_PER_AGENT: usize = 500;
const COMPACTION_TRIGGER_RATIO: f64 = 0.8;
const COMPACTION_RETAIN_RATIO: f64 = 0.6;

pub struct ToolMemoryEntry {
    pub name: String,
    pub description: String,
    pub category: Option<String>,
}

pub struct SkillMemoryEntry {
    pub name: String,
    pub description: String,
    pub when_to_use: String,
    pub allowed_tools: Vec<String>,
    pub user_invocable: bool,
}

/// 计算记忆重要性评分 — 基于事实类型、来源类型、标签和文本长度
pub fn calc_importance_score(fact_type: &str, source_type: &str, tags: Option<&str>, text_len: usize) -> f64 {
    let mut score: f64 = 1.0;

    match fact_type {
        "world" => score += 2.0,
        "mental_model" => score += 1.5,
        "experience" => score += 0.5,
        _ => {}
    }

    match source_type {
        "tool_init" => score += 3.0,
        "compaction" => score += 1.0,
        "tool_output" => score -= 0.3,
        _ => {}
    }

    if let Some(t) = tags {
        if t == TOOL_MEMORY_TAG || t == SKILL_MEMORY_TAG {
            score += 4.0;
        }
    }

    if text_len < 50 {
        score -= 0.5;
    } else if text_len > 500 {
        score += 0.3;
    }

    score.max(0.1).min(5.0)
}

#[allow(dead_code)]
/// 确定记忆分层 — 根据重要性评分和来源类型分配到working/episodic/semantic层
fn determine_memory_tier(importance_score: f64, source_type: &str, tags: Option<&str>) -> String {
    if let Some(t) = tags {
        if t == TOOL_MEMORY_TAG || t == SKILL_MEMORY_TAG {
            return MEMORY_TIER_CORE.to_string();
        }
    }
    if source_type == "tool_init" {
        return MEMORY_TIER_CORE.to_string();
    }
    if importance_score >= 3.5 {
        return MEMORY_TIER_IMPORTANT.to_string();
    }
    if importance_score >= 1.5 {
        return MEMORY_TIER_NORMAL.to_string();
    }
    MEMORY_TIER_EPHEMERAL.to_string()
}

/// 初始化工具和技能记忆 — 将工具/技能描述存入记忆系统供检索
pub async fn initialize_tool_skill_memories(tools: &[ToolMemoryEntry], skills: &[SkillMemoryEntry]) -> Result<usize, String> {
    let db = get_db().await;
    let mut count = 0;

    let existing_tool_memories: Vec<String> = match claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::AgentId.eq(SYSTEM_AGENT_ID))
        .filter(claw_db::db::entities::memory_units::Column::SourceType.eq("tool_init"))
        .all(db).await
    {
        Ok(records) => records.iter().map(|r| r.text.clone()).collect(),
        Err(e) => {
            log::warn!("[RAG:MemoryInit] Failed to query existing tool memories: {}", e);
            Vec::new()
        }
    };

    for tool in tools {
        let api_name = tool.name.replace(':', "_").replace('.', "_").replace(' ', "_");
        let memory_text = format!(
            "[Tool Knowledge] Name: {} | Description: {} | Category: {} | Usage: This tool can be invoked by name '{}' in the tools parameter. {}",
            api_name,
            tool.description,
            tool.category.as_deref().unwrap_or("general"),
            api_name,
            if tool.name.starts_with("Skill:") { "This is a skill-based tool, invoked via the Skill system." } else { "" }
        );

        if existing_tool_memories.iter().any(|m| m.contains(&format!("Name: {}", api_name))) {
            continue;
        }

        match store_enhanced_memory(
            SYSTEM_AGENT_ID,
            None,
            &memory_text,
            "world",
            "tool_init",
            Some(&format!("tool:{}", tool.name)),
            Some(TOOL_MEMORY_TAG),
        ).await {
            Ok(_) => count += 1,
            Err(e) => log::warn!("[RAG:MemoryInit] Failed to store memory for tool '{}': {}", tool.name, e),
        }
    }

    for skill in skills {
        let api_skill_name = format!("Skill_{}", skill.name);
        let memory_text = format!(
            "[Skill Knowledge] Name: {} | Description: {} | When to use: {} | Allowed tools: {} | Usage: Invoke via '{}' tool name. {}",
            api_skill_name,
            skill.description,
            skill.when_to_use,
            skill.allowed_tools.join(", "),
            api_skill_name,
            if skill.user_invocable { "User can directly invoke this skill." } else { "System-managed skill." }
        );

        if existing_tool_memories.iter().any(|m| m.contains(&format!("Name: {}", api_skill_name))) {
            continue;
        }

        match store_enhanced_memory(
            SYSTEM_AGENT_ID,
            None,
            &memory_text,
            "world",
            "tool_init",
            Some(&format!("skill:{}", skill.name)),
            Some(SKILL_MEMORY_TAG),
        ).await {
            Ok(_) => count += 1,
            Err(e) => log::warn!("[RAG:MemoryInit] Failed to store memory for skill '{}': {}", skill.name, e),
        }
    }

    log::info!("[RAG:MemoryInit] Initialized {} tool/skill memory entries", count);
    Ok(count)
}

/// 添加工具记忆
pub async fn add_tool_memory(tool_name: &str, description: &str, tool_type: &str) -> Result<String, String> {
    let memory_text = format!(
        "[Tool Knowledge] Name: {} | Description: {} | Type: {} | Usage: This tool can be invoked by name '{}' in the tools parameter.",
        tool_name, description, tool_type, tool_name
    );

    store_enhanced_memory(
        SYSTEM_AGENT_ID,
        None,
        &memory_text,
        "world",
        "tool_init",
        Some(&format!("{}:{}", tool_type, tool_name)),
        Some(TOOL_MEMORY_TAG),
    ).await
}

/// 添加技能记忆
pub async fn add_skill_memory(skill_name: &str, description: &str, when_to_use: &str, allowed_tools: &[String]) -> Result<String, String> {
    let api_skill_name = format!("Skill_{}", skill_name);
    let memory_text = format!(
        "[Skill Knowledge] Name: {} | Description: {} | When to use: {} | Allowed tools: {} | Usage: Invoke via '{}' tool name.",
        api_skill_name, description, when_to_use, allowed_tools.join(", "), api_skill_name
    );

    store_enhanced_memory(
        SYSTEM_AGENT_ID,
        None,
        &memory_text,
        "world",
        "tool_init",
        Some(&format!("skill:{}", skill_name)),
        Some(SKILL_MEMORY_TAG),
    ).await
}

/// 检索工具/技能相关上下文 — 根据查询匹配最相关的工具和技能
pub async fn retrieve_tool_skill_context(query: &str, limit: usize) -> Result<Vec<(String, f64)>, String> {
    let results = hybrid_retrieve(
        query,
        SYSTEM_AGENT_ID,
        None,
        limit,
    ).await?;

    Ok(results.iter()
        .filter(|r| r.tags.as_deref() == Some(TOOL_MEMORY_TAG) || r.tags.as_deref() == Some(SKILL_MEMORY_TAG))
        .map(|r| (r.text.clone(), r.final_score))
        .collect())
}

// ==================== 记忆压缩系统 ====================

pub struct MemoryCompactionStats {
    pub total_before: usize,
    pub total_after: usize,
    pub compressed: usize,
    pub deleted: usize,
    pub protected: usize,
}

/// 压缩指定Agent的记忆 — 按层合并、摘要、降级
pub async fn compact_memories_for_agent(agent_id: &str) -> Result<MemoryCompactionStats, String> {
    let db = get_db().await;
    let now = chrono::Utc::now().timestamp();

    let all_units = claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::AgentId.eq(agent_id))
        .all(db).await
        .map_err(|e| e.to_string())?;

    let total_before = all_units.len();

    if total_before < (MAX_MEMORY_UNITS_PER_AGENT as f64 * COMPACTION_TRIGGER_RATIO) as usize {
        return Ok(MemoryCompactionStats {
            total_before,
            total_after: total_before,
            compressed: 0,
            deleted: 0,
            protected: 0,
        });
    }

    let mut protected: Vec<_> = Vec::new();
    let mut compactable: Vec<_> = Vec::new();

    for unit in &all_units {
        let is_core = unit.tags.as_deref() == Some(TOOL_MEMORY_TAG)
            || unit.tags.as_deref() == Some(SKILL_MEMORY_TAG)
            || unit.source_type == "tool_init";
        let is_important = unit.importance_score >= 3.0;
        let is_recent = unit.occurred_at.map_or(false, |t| (now - t) < 86400 * 7);

        if is_core || is_important || is_recent {
            protected.push(unit);
        } else {
            compactable.push(unit);
        }
    }

    let target_count = (MAX_MEMORY_UNITS_PER_AGENT as f64 * COMPACTION_RETAIN_RATIO) as usize;
    let to_delete_count = if total_before > target_count {
        total_before.saturating_sub(target_count)
    } else {
        0
    };

    let mut to_delete: Vec<&claw_db::db::entities::memory_units::Model> = Vec::new();
    let mut to_compress: Vec<Vec<&claw_db::db::entities::memory_units::Model>> = Vec::new();

    let mut sorted_compactable = compactable.clone();
    sorted_compactable.sort_by(|a, b| {
        let score_a = a.importance_score * a.access_count as f64;
        let score_b = b.importance_score * b.access_count as f64;
        score_a.partial_cmp(&score_b).unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut remaining_delete = to_delete_count;
    let mut current_group: Vec<&claw_db::db::entities::memory_units::Model> = Vec::new();

    for unit in &sorted_compactable {
        if remaining_delete > 0 {
            to_delete.push(unit);
            remaining_delete -= 1;
        } else {
            current_group.push(unit);
            if current_group.len() >= 5 {
                to_compress.push(current_group.clone());
                current_group.clear();
            }
        }
    }
    if !current_group.is_empty() {
        to_compress.push(current_group);
    }

    let mut deleted = 0;
    for unit in &to_delete {
        delete_memory_unit(&unit.id).await?;
        deleted += 1;
    }

    let mut compressed = 0;
    for group in &to_compress {
        let combined_text: String = group.iter()
            .map(|u| u.text.as_str())
            .collect::<Vec<&str>>()
            .join("\n");

        let summary = summarize_memory_group(&combined_text);

        let _avg_importance = group.iter().map(|u| u.importance_score).sum::<f64>() / group.len() as f64;
        let tags = group.iter()
            .filter_map(|u| u.tags.clone())
            .next()
            .unwrap_or_default();

        store_enhanced_memory(
            agent_id,
            None,
            &summary,
            "experience",
            "compaction",
            Some(&format!("compressed_from_{}_units", group.len())),
            if tags.is_empty() { None } else { Some(&tags) },
        ).await.ok();

        for unit in group {
            delete_memory_unit(&unit.id).await.ok();
            compressed += 1;
        }
    }

    let total_after = total_before.saturating_sub(deleted).saturating_sub(compressed).saturating_add(to_compress.len());

    log::info!(
        "[RAG:Compaction] Agent '{}': {} → {} (deleted={}, compressed={}, protected={})",
        claw_types::truncate_str_safe(agent_id, 8),
        total_before, total_after, deleted, compressed, protected.len()
    );

    Ok(MemoryCompactionStats {
        total_before,
        total_after,
        compressed,
        deleted,
        protected: protected.len(),
    })
}

/// 压缩所有Agent的记忆
pub async fn compact_all_agents() -> Result<usize, String> {
    let db = get_db().await;

    let agent_ids: Vec<String> = claw_db::db::entities::memory_units::Entity::find()
        .select_only()
        .column(claw_db::db::entities::memory_units::Column::AgentId)
        .group_by(claw_db::db::entities::memory_units::Column::AgentId)
        .into_tuple::<String>()
        .all(db).await
        .map_err(|e| e.to_string())?;

    let mut total_compacted = 0;
    for agent_id in &agent_ids {
        match compact_memories_for_agent(agent_id).await {
            Ok(stats) => {
                if stats.compressed > 0 || stats.deleted > 0 {
                    total_compacted += 1;
                }
            }
            Err(e) => log::warn!("[RAG:Compaction] Failed for agent '{}': {}", claw_types::truncate_str_safe(agent_id, 8), e),
        }
    }

    log::info!("[RAG:Compaction] Compacted {} agents", total_compacted);
    Ok(total_compacted)
}

/// 删除单个记忆单元
async fn delete_memory_unit(id: &str) -> Result<(), String> {
    let db = get_db().await;

    let _ = db.execute(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM memory_vectors WHERE memory_unit_id = ?1",
        [id.into()],
    )).await;

    let _ = db.execute(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM memory_units_fts WHERE rowid = ?1",
        [id.into()],
    )).await;

    let _ = db.execute(sea_orm::Statement::from_sql_and_values(
        db.get_database_backend(),
        "DELETE FROM unit_entities WHERE unit_id = ?1",
        [id.into()],
    )).await;

    claw_db::db::entities::memory_units::Entity::delete_by_id(id.to_string())
        .exec(db).await
        .map_err(|e| e.to_string())?;

    Ok(())
}

/// 记忆组摘要 — 截断过长文本并添加摘要标记
fn summarize_memory_group(text: &str) -> String {
    let sentences: Vec<&str> = text.split(|c: char| c == '\n' || c == '.')
        .map(|s| s.trim())
        .filter(|s| s.len() > 10)
        .collect();

    if sentences.is_empty() {
        return format!("[Compressed] {}", claw_types::truncate_str_safe(&text, 200));
    }

    let mut keywords = std::collections::HashMap::new();
    for sentence in &sentences {
        for word in sentence.split_whitespace() {
            let w = word.to_lowercase();
            let w: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            if w.len() > 3 {
                *keywords.entry(w).or_insert(0) += 1;
            }
        }
    }

    let mut top_keywords: Vec<_> = keywords.iter().collect();
    top_keywords.sort_by(|a, b| b.1.cmp(a.1));
    let top_kw: Vec<&str> = top_keywords.iter().take(8).map(|(k, _)| k.as_str()).collect();

    let key_points: Vec<&str> = sentences.iter()
        .filter(|s| {
            top_kw.iter().any(|kw| s.to_lowercase().contains(*kw))
        })
        .take(5)
        .cloned()
        .collect();

    if key_points.is_empty() {
        format!("[Compressed Summary] Key topics: {}. {}",
            top_kw.join(", "),
            &sentences[0]
        )
    } else {
        format!("[Compressed Summary] Key topics: {}\n{}",
            top_kw.join(", "),
            key_points.join(". ")
        )
    }
}

/// 清理过时的工具/技能记忆 — 删除不再存在的工具和技能的记忆
pub async fn cleanup_stale_tool_memories(current_tools: &[ToolMemoryEntry], current_skills: &[SkillMemoryEntry]) -> Result<usize, String> {
    let db = get_db().await;

    let tool_memories = claw_db::db::entities::memory_units::Entity::find()
        .filter(claw_db::db::entities::memory_units::Column::AgentId.eq(SYSTEM_AGENT_ID))
        .filter(claw_db::db::entities::memory_units::Column::SourceType.eq("tool_init"))
        .all(db).await
        .map_err(|e| e.to_string())?;

    let current_names: Vec<String> = current_tools.iter().map(|t| t.name.clone())
        .chain(current_skills.iter().map(|s| s.name.clone()))
        .collect();

    let mut removed = 0;
    for mem in &tool_memories {
        let is_stale = !current_names.iter().any(|name| mem.text.contains(&format!("Name: {}", name)));
        if is_stale {
            delete_memory_unit(&mem.id).await.ok();
            removed += 1;
        }
    }

    if removed > 0 {
        log::info!("[RAG:MemoryCleanup] Removed {} stale tool/skill memories", removed);
    }
    Ok(removed)
}
