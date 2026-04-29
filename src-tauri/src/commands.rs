// Claw Desktop Commands - 核心 Tauri 命令处理函数
// 使用 State<ClawAppState> 统一状态管理模式

use crate::app_state::ClawAppState;
use claw_config::config::AppConfig;
use claw_db::database::Database;

// ==================== 配置相关命令 ====================

/// 获取当前应用配置（从 ClawAppState 统一状态读取）
#[tauri::command]
pub fn get_config(state: tauri::State<'_, ClawAppState>) -> Result<AppConfig, String> {
    Ok(state.get_config())
}

/// 保存配置到 config.toml 文件，同时更新内存状态
#[tauri::command]
pub fn save_config(
    state: tauri::State<'_, ClawAppState>,
    config: AppConfig,
    _app: tauri::AppHandle,
) -> Result<(), String> {
    let cfg_path = claw_config::path_resolver::config_path();
    log::info!("[Commands:save_config] Saving to {}", cfg_path.display());
    let parent = cfg_path.parent().ok_or("config path has no parent directory")?;
    std::fs::create_dir_all(parent).map_err(|e| format!("Failed to create dir {}: {}", parent.display(), e))?;
    config.save(parent).map_err(|e| format!("Save failed (path={}): {}", cfg_path.display(), e))?;

    *state.config.lock().expect("AppConfig lock poisoned") = config;
    Ok(())
}

// ==================== 会话管理命令 ====================

/// 获取所有会话列表（按更新时间倒序）
#[tauri::command]
pub async fn list_conversations() -> Result<Vec<serde_json::Value>, String> {
    let convs = Database::list_conversations().await.map_err(|e| e.to_string())?;
    Ok(convs.into_iter().map(|c| serde_json::to_value(c).unwrap_or(serde_json::json!({}))).collect())
}

/// 创建新会话，返回包含 ID 和时间戳的 JSON 对象
#[tauri::command]
pub async fn create_conversation(agent_id: Option<String>) -> Result<serde_json::Value, String> {
    let conv = Database::create_conversation(agent_id).await.map_err(|e| e.to_string())?;
    Ok(serde_json::to_value(conv).unwrap_or(serde_json::Value::Null))
}

/// 获取指定会话的所有消息（按时间正序）
#[tauri::command]
pub async fn get_messages(conversation_id: String) -> Result<Vec<serde_json::Value>, String> {
    let msgs = Database::get_messages(&conversation_id).await.map_err(|e| e.to_string())?;
    Ok(msgs.into_iter().map(|m| serde_json::to_value(m).unwrap_or(serde_json::json!({}))).collect())
}

/// 核心命令：发送用户消息 → 存储用户消息 → 调用 LLM API(含工具循环) → 存储助手回复 → 返回结果
#[tauri::command]
pub async fn send_message(
    state: tauri::State<'_, ClawAppState>,
    conversation_id: String,
    content: String,
    images: Option<Vec<serde_json::Value>>,
) -> Result<serde_json::Value, String> {
    use claw_llm::llm::*;

    let config = state.get_config();

    Database::add_message(&conversation_id, "user", &content, None, None, None).await.map_err(|e| e.to_string())?;

    let response: ChatResponse = send_chat_message(&config, &conversation_id, &content, images.as_deref()).await.map_err(|e| e.to_string())?;

    let result = build_send_message_result(response, &config.model.default_model);

    Database::add_message(&conversation_id, "assistant", &result.reply_text, None, result.total_tokens, result.metadata_str).await.map_err(|e| e.to_string())?;
    if result.reply_text != "(no reply)" {
        let title = if content.len() > 50 {
            let mut end = 50;
            while end > 0 && !content.is_char_boundary(end) { end -= 1; }
            format!("{}...", &content[..end])
        } else { content.clone() };
        let _ = Database::rename_conversation(&conversation_id, &title).await;
    }

    Ok(serde_json::json!({
        "text": result.reply_text,
        "usage": result.usage,
        "tool_calls": result.tool_calls.iter().map(|t| serde_json::json!({"id": t.id, "name": t.name, "input": t.input})).collect::<Vec<_>>(),
        "tool_executions": result.tool_executions.iter().map(|e| serde_json::json!({"round": e.round, "tool_name": e.tool_name, "tool_input": e.tool_input, "tool_result": e.tool_result, "duration_ms": e.duration_ms})).collect::<Vec<_>>(),
        "streamed": false
    }))
}
