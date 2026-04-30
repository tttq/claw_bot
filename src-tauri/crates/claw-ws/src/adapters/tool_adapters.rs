// Claw Desktop - 工具适配器 - 将工具调用适配为WS事件
use claw_tools::agent_session;
use claw_tools::extension_manager;
use claw_tools::plugins::agent as agent_tools;
use claw_tools::plugins::file as file_tools;
use claw_tools::plugins::git as git_tools;
use claw_tools::plugins::misc as misc_tools;
use claw_tools::plugins::search as search_tools;
use claw_tools::plugins::shell as shell_tools;
use claw_tools::plugins::web as web_tools;
use claw_tools::skill_loader;
use claw_tools::tool_registry;

fn extract_string(params: &serde_json::Value, key: &str) -> Option<String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
}

fn extract_agent_id(params: &serde_json::Value) -> Result<String, String> {
    extract_string(params, "agentId")
        .or(extract_string(params, "agent_id"))
        .or(extract_string(params, "id"))
        .ok_or("Missing agentId/agent_id/id".to_string())
}

fn extract_value(params: &serde_json::Value, key: &str) -> Option<serde_json::Value> {
    params.get(key).cloned()
}

fn extract_u64(params: &serde_json::Value, key: &str) -> Option<u64> {
    params.get(key).and_then(|v| v.as_u64())
}

fn extract_bool(params: &serde_json::Value, key: &str) -> Option<bool> {
    params.get(key).and_then(|v| v.as_bool())
}

// ==================== Shell Tools ====================

pub async fn tool_bash_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let command = extract_string(params, "command").ok_or("Missing command")?;
    let working_dir = extract_string(params, "working_dir");
    let timeout_secs = extract_u64(params, "timeout_secs");
    shell_tools::tool_bash(command, working_dir, timeout_secs).await
}

pub async fn tool_bash_cancel_ws(_params: &serde_json::Value) -> Result<serde_json::Value, String> {
    shell_tools::tool_bash_cancel()
}

// ==================== Search Tools ====================

pub async fn tool_glob_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let pattern = extract_string(params, "pattern").ok_or("Missing pattern")?;
    let path = extract_string(params, "path");
    let exclude_patterns: Option<Vec<String>> = params
        .get("exclude_patterns")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    search_tools::tool_glob(pattern, path, exclude_patterns)
}

pub async fn tool_grep_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let pattern = extract_string(params, "pattern").ok_or("Missing pattern")?;
    let path = extract_string(params, "path");
    let include_pattern = extract_string(params, "include_pattern");
    let exclude_pattern = extract_string(params, "exclude_pattern");
    search_tools::tool_grep(pattern, path, include_pattern, exclude_pattern)
}

// ==================== File Tools (via file_tools module) ====================

pub async fn file_read_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    file_tools::tool_read_ws(params).await
}

pub async fn file_edit_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    file_tools::tool_edit_ws(params).await
}

pub async fn file_write_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    file_tools::tool_write_ws(params).await
}

// ==================== Web Tools ====================

pub async fn tool_web_fetch_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let url = extract_string(params, "url").ok_or("Missing url")?;
    let max_length = extract_u64(params, "max_length");
    web_tools::tool_web_fetch(url, max_length).await
}

pub async fn tool_web_search_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = extract_string(params, "query").ok_or("Missing query")?;
    let engine = extract_string(params, "engine");
    let num_results = extract_u64(params, "num_results");
    let allowed_domains = extract_value(params, "allowed_domains");
    let blocked_domains = extract_value(params, "blocked_domains");
    web_tools::tool_web_search(query, engine, num_results, allowed_domains, blocked_domains).await
}

// ==================== Agent Tools ====================

pub async fn tool_agent_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let prompt = extract_string(params, "prompt").ok_or("Missing prompt")?;
    let mode = extract_string(params, "mode");
    let model_override = extract_string(params, "model_override");
    let agent_id = extract_string(params, "agent_id");
    agent_tools::tool_agent(prompt, mode, model_override, agent_id).await
}

pub async fn tool_todo_write_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let todos = extract_value(params, "todos").ok_or("Missing todos")?;
    agent_tools::tool_todo_write(todos)
}

pub async fn tool_todo_get_ws(_params: &serde_json::Value) -> Result<serde_json::Value, String> {
    agent_tools::tool_todo_get()
}

pub async fn tool_task_create_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let prompt = extract_string(params, "prompt").ok_or("Missing prompt")?;
    agent_tools::tool_task_create(prompt)
}

pub async fn tool_task_get_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let task_id = extract_string(params, "task_id").ok_or("Missing task_id")?;
    agent_tools::tool_task_get(task_id)
}

pub async fn tool_task_update_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let task_id = extract_string(params, "task_id").ok_or("Missing task_id")?;
    let status = extract_string(params, "status").ok_or("Missing status")?;
    agent_tools::tool_task_update(task_id, status)
}

pub async fn tool_task_list_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let status_filter = extract_string(params, "status_filter");
    agent_tools::tool_task_list(status_filter)
}

pub async fn tool_workflow_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "name").ok_or("Missing name")?;
    let steps = extract_value(params, "steps");
    let inputs = extract_value(params, "inputs");
    agent_tools::tool_workflow(name, steps, inputs)
}

pub async fn tool_skill_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let skill_name = extract_string(params, "skill_name").ok_or("Missing skill_name")?;
    let args = extract_value(params, "args");
    agent_tools::tool_skill(skill_name, args)
}

pub async fn tool_brief_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let message = extract_string(params, "message").ok_or("Missing message")?;
    let attachments: Option<Vec<String>> = params
        .get("attachments")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    agent_tools::tool_brief(message, attachments)
}

pub async fn tool_config_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let action = extract_string(params, "action").ok_or("Missing action")?;
    let key = extract_string(params, "key");
    let value = extract_value(params, "value");
    agent_tools::tool_config(action, key, value)
}

pub async fn tool_notebook_edit_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let file_path = extract_string(params, "file_path").ok_or("Missing file_path")?;
    let cell_index = extract_u64(params, "cell_index").ok_or("Missing cell_index")? as u64;
    let source = extract_value(params, "source");
    agent_tools::tool_notebook_edit(file_path, cell_index, source)
}

pub async fn tool_schedule_cron_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "name").ok_or("Missing name")?;
    let schedule = extract_string(params, "schedule").ok_or("Missing schedule")?;
    let task = extract_string(params, "task").ok_or("Missing task")?;
    let enabled = extract_bool(params, "enabled");
    agent_tools::tool_schedule_cron(name, schedule, task, enabled)
}

pub async fn tool_schedule_list_ws() -> Result<serde_json::Value, String> {
    agent_tools::tool_schedule_list()
}

pub async fn tool_ask_user_question_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let questions = extract_value(params, "questions").ok_or("Missing questions")?;
    agent_tools::tool_ask_user_question(questions)
}

pub async fn tool_tool_search_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let query = extract_string(params, "query").ok_or("Missing query")?;
    let max_results = extract_u64(params, "max_results");
    agent_tools::tool_tool_search(query, max_results)
}

// ==================== Misc Tools ====================

pub async fn get_env_variables_ws() -> Result<serde_json::Value, String> {
    misc_tools::get_env_variables(None)
}

pub async fn get_env_session_info_ws() -> Result<serde_json::Value, String> {
    misc_tools::get_env_session_info()
}

pub async fn get_code_changes_summary_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    let staged_only = extract_bool(params, "staged_only");
    misc_tools::get_code_changes_summary(working_dir, staged_only)
}

pub async fn run_code_review_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let config_value = params.get("config").ok_or("Missing config parameter")?;
    let config: claw_config::config::AppConfig = serde_json::from_value(config_value.clone())
        .map_err(|e| format!("Invalid config: {}", e))?;
    let changes_summary =
        extract_value(params, "changes_summary").ok_or("Missing changes_summary")?;
    misc_tools::run_code_review(config, changes_summary).await
}

pub async fn toggle_fast_mode_ws() -> Result<serde_json::Value, String> {
    misc_tools::toggle_fast_mode(true)
}

// ==================== Git Tools ====================

pub async fn git_status_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_status(working_dir)
}

pub async fn git_diff_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    let file_path = extract_string(params, "file_path");
    let staged = extract_bool(params, "staged");
    git_tools::git_diff(working_dir, file_path, staged)
}

pub async fn git_commit_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let message = extract_string(params, "message").ok_or("Missing message")?;
    let files: Option<Vec<String>> = params
        .get("files")
        .and_then(|v| serde_json::from_value(v.clone()).ok());
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_commit(message, files, working_dir)
}

pub async fn git_log_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let limit = extract_u64(params, "limit").or_else(|| extract_u64(params, "max_count"));
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_log(limit, working_dir)
}

pub async fn git_branch_list_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_branch_list(working_dir)
}

pub async fn git_create_branch_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let branch_name = extract_string(params, "branch_name")
        .or(extract_string(params, "name"))
        .ok_or("Missing branch_name")?;
    let checkout = extract_bool(params, "checkout");
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_create_branch(branch_name, checkout, working_dir)
}

pub async fn git_checkout_branch_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let branch_name = extract_string(params, "branch_name")
        .or(extract_string(params, "name"))
        .ok_or("Missing branch_name")?;
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_checkout_branch(branch_name, working_dir)
}

pub async fn git_add_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let files: Vec<String> = params
        .get("files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_add(files, working_dir)
}

pub async fn git_reset_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let files: Vec<String> = params
        .get("files")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .unwrap_or_default();
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_reset(files, working_dir)
}

// ==================== Extension Commands ====================

pub async fn cmd_install_extension_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let url = extract_string(params, "url").ok_or("Missing url")?;
    let name = extract_string(params, "name");
    extension_manager::cmd_install_extension(url, name)
}

pub async fn cmd_uninstall_extension_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "name").ok_or("Missing name")?;
    extension_manager::cmd_uninstall_extension(name)
}

// ==================== Missing Git Adapters ====================

pub async fn git_stash_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_stash(working_dir)
}

pub async fn git_stash_pop_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_stash_pop(working_dir)
}

pub async fn git_is_repository_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let working_dir = extract_string(params, "working_dir");
    git_tools::git_is_repository(working_dir).map(|b| serde_json::json!({ "is_repository": b }))
}

// ==================== Missing Tool Listing ====================

pub async fn list_all_tools_ws() -> serde_json::Value {
    let tools = tool_registry::list_all_tools().await;
    serde_json::json!({ "count": tools.len(), "tools": tools })
}

// ==================== Missing CMD Adapters ====================

pub async fn cmd_list_all_tools_ws() -> Result<serde_json::Value, String> {
    tool_registry::cmd_list_all_tools().await
}

pub async fn cmd_register_tool_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "name").ok_or("Missing name")?;
    let description = extract_string(params, "description").ok_or("Missing description")?;
    let input_schema = extract_value(params, "input_schema").ok_or("Missing input_schema")?;
    let handler = extract_string(params, "handler");
    tool_registry::cmd_register_tool(name, description, input_schema, handler).await
}

pub async fn cmd_unregister_tool_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "name").ok_or("Missing name")?;
    tool_registry::cmd_unregister_tool(name).await
}

pub async fn cmd_load_skills_from_dir_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let dir = extract_string(params, "dir").ok_or("Missing dir")?;
    let source = extract_string(params, "source");
    skill_loader::cmd_load_skills_from_dir(dir, source).await
}

pub async fn cmd_list_loaded_skills_ws() -> Result<serde_json::Value, String> {
    skill_loader::cmd_list_loaded_skills().await
}

pub async fn cmd_scan_extensions_ws() -> Result<serde_json::Value, String> {
    extension_manager::cmd_scan_extensions().await
}

// ==================== Missing ISO Agent Adapters ====================

pub async fn iso_agent_list_ws(_params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let agents = agent_session::iso_agent_list().await?;
    Ok(serde_json::json!({ "count": agents.len(), "agents": agents }))
}

pub async fn iso_agent_create_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "displayName")
        .or(extract_string(params, "name"))
        .or(extract_string(params, "display_name"))
        .ok_or("Missing displayName/name/display_name")?;
    let description = extract_string(params, "description").unwrap_or_default();
    let system_prompt = extract_string(params, "systemPrompt")
        .or(extract_string(params, "system_prompt"))
        .ok_or("Missing systemPrompt/system_prompt")?;
    let purpose = extract_string(params, "purpose");
    let scope = extract_string(params, "scope");
    let category = extract_string(params, "category");
    let model_override =
        extract_string(params, "modelOverride").or(extract_string(params, "model_override"));
    let agent = agent_session::iso_agent_create(
        name,
        description,
        system_prompt,
        purpose,
        scope,
        category,
        model_override,
    )
    .await?;
    Ok(serde_json::json!(agent))
}

pub async fn iso_agent_get_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = extract_agent_id(params)?;
    let agent = agent_session::iso_agent_get(id).await?;
    Ok(serde_json::json!(agent))
}

pub async fn iso_agent_rename_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = extract_agent_id(params)?;
    let new_name = extract_string(params, "newName")
        .or(extract_string(params, "new_name"))
        .ok_or("Missing newName")?;
    let agent = agent_session::iso_agent_rename(id, new_name).await?;
    Ok(serde_json::json!(agent))
}

pub async fn iso_agent_delete_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let id = extract_agent_id(params)?;
    agent_session::iso_agent_delete(id).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_set_config_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let key = extract_string(params, "key").ok_or("Missing key")?;
    let value = extract_string(params, "value").ok_or("Missing value")?;
    agent_session::iso_set_config(agent_id, key, value).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_get_config_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let key = extract_string(params, "key").unwrap_or_default();
    let value = agent_session::iso_get_config(agent_id, key).await?;
    Ok(serde_json::json!({ "value": value }))
}

pub async fn iso_init_agent_db_ws(
    _params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({ "success": true, "note": "DB init handled at startup" }))
}

pub async fn iso_set_tools_config_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let config = extract_value(params, "config").ok_or("Missing config")?;
    agent_session::iso_set_tools_config(agent_id, config).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_set_skills_enabled_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let enabled: Vec<String> = params
        .get("enabled")
        .and_then(|v| serde_json::from_value(v.clone()).ok())
        .ok_or("Missing enabled")?;
    agent_session::iso_set_skills_enabled(agent_id, enabled).await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_agent_update_config_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let system_prompt =
        extract_string(params, "systemPrompt").or(extract_string(params, "system_prompt"));
    let purpose = extract_string(params, "purpose");
    let scope = extract_string(params, "scope");
    let model_override =
        extract_string(params, "modelOverride").or(extract_string(params, "model_override"));
    let max_turns = extract_u64(params, "maxTurns")
        .or_else(|| extract_u64(params, "max_turns"))
        .map(|v| v as u32);
    let temperature = params.get("temperature").and_then(|v| v.as_f64());
    agent_session::iso_agent_update_config(
        agent_id,
        system_prompt,
        purpose,
        scope,
        model_override,
        max_turns,
        temperature,
    )
    .await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_create_session_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let conversation_id =
        extract_string(params, "conversationId").or(extract_string(params, "conversation_id"));
    let session = agent_session::iso_create_session(agent_id, conversation_id).await?;
    Ok(serde_json::json!(session))
}

pub async fn iso_list_sessions_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let sessions = agent_session::iso_list_sessions(agent_id).await?;
    Ok(serde_json::json!({ "count": sessions.len(), "sessions": sessions }))
}

pub async fn iso_index_workspace_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let relative_path = extract_string(params, "relativePath")
        .or(extract_string(params, "relative_path"))
        .or(extract_string(params, "path"))
        .ok_or("Missing relativePath/path")?;
    let full_path = extract_string(params, "fullPath")
        .or(extract_string(params, "full_path"))
        .unwrap_or_default();
    let file_size = params
        .get("fileSize")
        .or(params.get("file_size"))
        .and_then(|v| v.as_i64())
        .unwrap_or(0);
    let content_type =
        extract_string(params, "contentType").or(extract_string(params, "content_type"));
    agent_session::iso_index_workspace(agent_id, relative_path, full_path, file_size, content_type)
        .await?;
    Ok(serde_json::json!({ "success": true }))
}

pub async fn iso_list_workspace_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let agent_id = extract_agent_id(params)?;
    let entries = agent_session::iso_list_workspace(agent_id).await?;
    Ok(serde_json::json!({ "count": entries.len(), "entries": entries }))
}

pub async fn iso_cleanup_ws(params: &serde_json::Value) -> Result<serde_json::Value, String> {
    let days_threshold = params
        .get("daysThreshold")
        .or(params.get("days_threshold"))
        .and_then(|v| v.as_i64())
        .unwrap_or(30);
    let count = agent_session::iso_cleanup(days_threshold).await?;
    Ok(serde_json::json!({ "cleaned": count }))
}

pub async fn iso_generate_prompt_ws(
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    let name = extract_string(params, "displayName")
        .or(extract_string(params, "name"))
        .unwrap_or_default();
    let category = extract_string(params, "category").unwrap_or_else(|| "general".to_string());
    let purpose = extract_string(params, "purpose").unwrap_or_default();
    let scope = extract_string(params, "scope").unwrap_or_default();
    let description = extract_string(params, "description").unwrap_or_default();

    let config_value = params
        .get("config")
        .ok_or("Missing config parameter — global model must be configured")?;
    let config: claw_config::config::AppConfig = serde_json::from_value(config_value.clone())
        .map_err(|e| format!("Invalid config: {}", e))?;

    let api_key = config.resolve_api_key().map_err(|e| e.to_string())?;
    if api_key.is_empty() {
        return Err("API key not configured — please set up a model in Settings first".to_string());
    }

    let category_desc = match category.as_str() {
        "code" => "programming and software development",
        "search" => "research, information retrieval, and web search",
        "analysis" => "data analysis, critical thinking, and decision support",
        "creative" => "creative writing, brainstorming, and content generation",
        _ => "general-purpose assistance",
    };

    let meta_prompt = format!(
        r#"Generate a concise, effective system prompt for an AI agent with the following characteristics:

- **Name**: {name}
- **Category**: {category} ({category_desc})
{purpose_section}{scope_section}{description_section}

Requirements:
1. Start with "You are {name}, ..." to establish identity
2. Focus on the agent's core expertise and capabilities
3. Include 3-5 specific behavioral guidelines relevant to the category
4. Keep it under 300 words — be concise and actionable
5. Do NOT include generic disclaimers or safety warnings (those are handled globally)
6. Write in English unless the name suggests another language
7. Output ONLY the system prompt text — no explanations, no markdown formatting around it"#,
        name = name,
        category = category,
        category_desc = category_desc,
        purpose_section = if purpose.is_empty() {
            String::new()
        } else {
            format!("- **Purpose**: {}\n", purpose)
        },
        scope_section = if scope.is_empty() {
            String::new()
        } else {
            format!("- **Scope**: {}\n", scope)
        },
        description_section = if description.is_empty() {
            String::new()
        } else {
            format!("- **Description**: {}\n", description)
        },
    );

    let messages = vec![
        serde_json::json!({
            "role": "system",
            "content": "You are a prompt engineering expert. Generate high-quality system prompts for AI agents. Output only the prompt text, nothing else."
        }),
        serde_json::json!({
            "role": "user",
            "content": meta_prompt
        }),
    ];

    let client = claw_llm::http_client();
    let base_url = config.get_base_url();
    let base_trimmed = base_url.trim_end_matches('/');

    let result = if config.is_openai_compatible() {
        let url = format!("{}/chat/completions", base_trimmed);
        let body = serde_json::json!({
            "model": config.model.default_model,
            "max_tokens": 1024,
            "temperature": 0.7,
            "stream": false,
            "messages": messages,
        });

        let response = client
            .post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!(
                "API error ({}): {}",
                status,
                claw_types::truncate_str_safe(&text, 200)
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        json.get("choices")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("message"))
            .and_then(|m| m.get("content"))
            .and_then(|c| c.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    } else {
        let url = format!("{}/v1/messages", base_trimmed);
        let body = serde_json::json!({
            "model": config.model.default_model,
            "max_tokens": 1024,
            "temperature": 0.7,
            "stream": false,
            "system": "You are a prompt engineering expert. Generate high-quality system prompts for AI agents. Output only the prompt text, nothing else.",
            "messages": messages.iter().filter(|m| m["role"] != "system").cloned().collect::<Vec<_>>(),
        });

        let response = client
            .post(&url)
            .header("x-api-key", &api_key)
            .header("anthropic-version", "2023-06-01")
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("API request failed: {}", e))?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            return Err(format!(
                "API error ({}): {}",
                status,
                claw_types::truncate_str_safe(&text, 200)
            ));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| format!("Failed to parse response: {}", e))?;
        json.get("content")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("text"))
            .and_then(|t| t.as_str())
            .unwrap_or("")
            .trim()
            .to_string()
    };

    if result.is_empty() {
        return Err("LLM returned empty response — please try again".to_string());
    }

    Ok(serde_json::json!({ "prompt": result }))
}

// ==================== Missing Memory V2 Adapters ====================

pub async fn memory_v2_list_entities_ws(agent_id: &str) -> Result<serde_json::Value, String> {
    let results = claw_rag::rag::hybrid_retrieve("", agent_id, None, 100)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "count": results.len(), "entities": results }))
}

pub async fn memory_v2_stats_ws(agent_id: &str) -> Result<serde_json::Value, String> {
    let results = claw_rag::rag::hybrid_retrieve("", agent_id, None, 1)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "agent_id": agent_id, "total_memories": results.len() }))
}

pub async fn memory_v2_delete_ws(unit_id: &str) -> Result<serde_json::Value, String> {
    claw_rag::rag::delete_conversation_memories(unit_id)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "success": true, "deleted": unit_id }))
}

pub async fn memory_v2_export_ws(agent_id: &str) -> Result<serde_json::Value, String> {
    let results = claw_rag::rag::hybrid_retrieve("", agent_id, None, 10000)
        .await
        .map_err(|e| e.to_string())?;
    Ok(serde_json::json!({ "agent_id": agent_id, "count": results.len(), "memories": results }))
}
