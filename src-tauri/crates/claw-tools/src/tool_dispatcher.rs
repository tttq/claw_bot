// Claw Desktop - 工具分发器 - 根据工具名称路由到对应的插件实现
// 支持 7 大类 38+ 工具的统一分发，包括 Shell、文件、搜索、Web、Git、浏览器、UI 自动化

use serde_json::json;

/// 从JSON参数中提取字符串值
fn extract_string(params: &serde_json::Value, key: &str) -> Result<String, String> {
    params
        .get(key)
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("missing required parameter: '{}'", key))
}

/// 直接启动应用降级 — 当UI自动化引擎失败时，尝试直接启动应用
async fn try_direct_launch_fallback(
    instruction: &str,
    executor: &dyn claw_traits::automation::AutomationExecutor,
) -> Option<serde_json::Value> {
    let lower = instruction.to_lowercase();
    let patterns = [
        "打开",
        "启动",
        "运行",
        "launch",
        "open",
        "start",
        "run",
        "帮我打开",
        "帮我启动",
    ];

    let mut last_end = 0usize;
    let mut found = false;
    for p in &patterns {
        if let Some(pos) = lower.find(p) {
            let end = pos + p.len();
            if end > last_end {
                last_end = end;
                found = true;
            }
        }
    }

    let query = if found {
        instruction[last_end..]
            .trim()
            .trim_matches(|c: char| c == ',' || c == '，' || c == '。' || c == '.')
            .to_string()
    } else {
        instruction.trim().to_string()
    };

    if query.is_empty() || query.len() < 2 {
        return None;
    }

    log::info!(
        "[ToolDispatcher:try_direct_launch_fallback] Trying direct launch for '{}'",
        query
    );

    match executor.launch_application(&query).await {
        Ok(_) => Some(json!({
            "tool":"ExecuteAutomation",
            "success":true,
            "output": format!("Launched '{}' via direct launch fallback", query)
        })),
        Err(e) => {
            log::warn!(
                "[ToolDispatcher:try_direct_launch_fallback] Direct launch failed: {}",
                e
            );
            None
        }
    }
}

/// 从JSON参数中提取u64值
fn extract_u64(params: &serde_json::Value, key: &str) -> Result<u64, String> {
    params
        .get(key)
        .and_then(|v| v.as_u64())
        .ok_or_else(|| format!("missing required parameter: '{}'", key))
}

/// 从JSON参数中提取布尔值
#[allow(dead_code)]
fn extract_bool(params: &serde_json::Value, key: &str) -> Result<bool, String> {
    params
        .get(key)
        .and_then(|v| v.as_bool())
        .ok_or_else(|| format!("missing required parameter: '{}'", key))
}

/// 工具分发器 — 根据工具名称路由到对应的插件实现
///
/// 支持7大类38+工具：Shell、文件、搜索、Web、Agent、Git、浏览器、UI自动化
/// 未知工具返回包含所有可用工具列表的错误提示
pub async fn dispatch_tool(
    name: &str,
    params: &serde_json::Value,
) -> Result<serde_json::Value, String> {
    log::info!(
        "[ToolDispatcher] 分发工具: {} | 参数数量: {}",
        name,
        params.as_object().map(|o| o.len()).unwrap_or(0)
    );

    match name {
        // ==================== Shell Tools ====================
        "bash" | "Bash" => {
            let command = extract_string(params, "command")?;
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let timeout_secs = params.get("timeout_secs").and_then(|v| v.as_u64());
            crate::plugins::shell::tool_bash(command, working_dir, timeout_secs).await
        }
        "bash_cancel" | "BashCancel" => crate::plugins::shell::tool_bash_cancel(),

        // ==================== File Tools ====================
        "file_read" | "Read" => {
            let file_path = extract_string(params, "file_path")?;
            let offset = params.get("offset").and_then(|v| v.as_u64());
            let limit = params.get("limit").and_then(|v| v.as_u64());
            crate::plugins::file::tool_read(file_path, offset, limit)
        }
        "file_edit" | "Edit" => {
            let file_path = extract_string(params, "file_path")?;
            let edits = params.get("edits").cloned().unwrap_or(json!([]));
            let dry_run = params.get("dry_run").and_then(|v| v.as_bool());
            crate::plugins::file::tool_edit(file_path, edits, dry_run)
        }
        "file_write" | "Write" => {
            let file_path = extract_string(params, "file_path")?;
            let content = extract_string(params, "content")?;
            let create_dirs = params.get("create_dirs").and_then(|v| v.as_bool());
            crate::plugins::file::tool_write(file_path, content, create_dirs)
        }

        // ==================== Search Tools ====================
        "glob" | "Glob" => {
            let pattern = extract_string(params, "pattern")?;
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let exclude_patterns = params
                .get("exclude_patterns")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            crate::plugins::search::tool_glob(pattern, path, exclude_patterns)
        }
        "grep" | "Grep" => {
            let pattern = extract_string(params, "pattern")?;
            let path = params
                .get("path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let include_pattern = params
                .get("include_pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let exclude_pattern = params
                .get("exclude_pattern")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::search::tool_grep(pattern, path, include_pattern, exclude_pattern)
        }

        // ==================== Web Tools ====================
        "web_fetch" | "WebFetch" | "fetch" => {
            let url = extract_string(params, "url")?;
            let max_length = params.get("max_length").and_then(|v| v.as_u64());
            crate::plugins::web::tool_web_fetch(url, max_length).await
        }
        "web_search" | "WebSearch" | "search" => {
            let query = extract_string(params, "query")?;
            let engine = params
                .get("engine")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let num_results = params.get("num_results").and_then(|v| v.as_u64());
            let allowed_domains = params.get("allowed_domains").cloned();
            let blocked_domains = params.get("blocked_domains").cloned();
            crate::plugins::web::tool_web_search(
                query,
                engine,
                num_results,
                allowed_domains,
                blocked_domains,
            )
            .await
        }

        // ==================== Agent Tools ====================
        "agent" | "Agent" => {
            let prompt = extract_string(params, "prompt")?;
            let mode = params
                .get("mode")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let model_override = params
                .get("model_override")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let agent_id = params
                .get("agent_id")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::agent::tool_agent(prompt, mode, model_override, agent_id).await
        }
        "todo_write" | "TodoWrite" => {
            let todos = params.get("todos").cloned().unwrap_or(json!([]));
            crate::plugins::agent::tool_todo_write(todos)
        }
        "todo_get" | "TodoGet" => crate::plugins::agent::tool_todo_get(),
        "task_create" | "TaskCreate" => {
            let prompt = extract_string(params, "prompt")?;
            crate::plugins::agent::tool_task_create(prompt)
        }
        "task_get" | "TaskGet" => {
            let task_id = extract_string(params, "task_id")?;
            crate::plugins::agent::tool_task_get(task_id)
        }
        "task_update" | "TaskUpdate" => {
            let task_id = extract_string(params, "task_id")?;
            let status = extract_string(params, "status")?;
            crate::plugins::agent::tool_task_update(task_id, status)
        }
        "task_list" | "TaskList" => {
            let status_filter = params
                .get("status_filter")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::agent::tool_task_list(status_filter)
        }
        "workflow" | "Workflow" => {
            let name = extract_string(params, "name")?;
            let steps = params.get("steps").cloned();
            let inputs = params.get("inputs").cloned();
            crate::plugins::agent::tool_workflow(name, steps, inputs)
        }
        "skill" | "Skill" => {
            let skill_name = extract_string(params, "skill_name")
                .or_else(|_| extract_string(params, "skill"))
                .unwrap_or_default();
            let args = params.get("args").cloned();
            crate::plugins::agent::tool_skill(skill_name, args)
        }
        name if name.starts_with("Skill:") => {
            let skill_name = name.strip_prefix("Skill:").unwrap_or("").to_string();
            let args = params.get("args").cloned();
            crate::plugins::agent::tool_skill(skill_name, args)
        }
        "enter_plan_mode" | "EnterPlanMode" => crate::plugins::agent::tool_enter_plan_mode(),
        "exit_plan_mode" | "ExitPlanMode" => crate::plugins::agent::tool_exit_plan_mode(),
        "get_plan_status" | "GetPlanStatus" => crate::plugins::agent::tool_get_plan_status(),
        "brief" | "Brief" => {
            let message = extract_string(params, "message")?;
            let attachments = params
                .get("attachments")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                });
            crate::plugins::agent::tool_brief(message, attachments)
        }
        "config" | "Config" => {
            let action = extract_string(params, "action")?;
            let key = params
                .get("key")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let value = params.get("value").cloned();
            crate::plugins::agent::tool_config(action, key, value)
        }
        "notebook_edit" | "NotebookEdit" => {
            let file_path = extract_string(params, "file_path")?;
            let cell_index = extract_u64(params, "cell_index")?;
            let source = params.get("source").cloned();
            crate::plugins::agent::tool_notebook_edit(file_path, cell_index, source)
        }
        "schedule_cron" | "ScheduleCron" => {
            let name = extract_string(params, "name")?;
            let schedule = extract_string(params, "schedule")?;
            let task = extract_string(params, "task")?;
            let enabled = params.get("enabled").and_then(|v| v.as_bool());
            crate::plugins::agent::tool_schedule_cron(name, schedule, task, enabled)
        }
        "schedule_list" | "ScheduleList" => crate::plugins::agent::tool_schedule_list(),
        "ask_user_question" | "AskUserQuestion" => {
            let questions = params.get("questions").cloned().unwrap_or(json!([]));
            crate::plugins::agent::tool_ask_user_question(questions)
        }
        "tool_search" | "ToolSearch" => {
            let query = extract_string(params, "query")?;
            let max_results = params.get("max_results").and_then(|v| v.as_u64());
            crate::plugins::agent::tool_tool_search(query, max_results)
        }

        // ==================== Dynamic Extension Tools ====================
        "create_tool" | "CreateTool" => {
            crate::plugins::agent::tool_create_tool(params.clone()).await
        }
        "create_skill" | "CreateSkill" => {
            crate::plugins::agent::tool_create_skill(params.clone()).await
        }

        // ==================== Git Tools ====================
        "git_status" | "GitStatus" => {
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_status(working_dir)
        }
        "git_diff" | "GitDiff" => {
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let file_path = params
                .get("file_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let staged = params.get("staged").and_then(|v| v.as_bool());
            crate::plugins::git::git_diff(working_dir, file_path, staged)
        }
        "git_commit" | "GitCommit" => {
            let message = extract_string(params, "message")?;
            let files = params.get("files").and_then(|v| v.as_array()).map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(|s| s.to_string()))
                    .collect()
            });
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_commit(message, files, working_dir)
        }
        "git_log" | "GitLog" => {
            let limit = params.get("limit").and_then(|v| v.as_u64());
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_log(limit, working_dir)
        }
        "git_branch" | "GitBranch" => {
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_branch_list(working_dir)
        }
        "git_create_branch" | "GitCreateBranch" => {
            let name = extract_string(params, "name")?;
            let checkout = params.get("checkout").and_then(|v| v.as_bool());
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_create_branch(name, checkout, working_dir)
        }
        "git_checkout" | "GitCheckout" => {
            let name = extract_string(params, "name")?;
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_checkout_branch(name, working_dir)
        }
        "git_stash" | "GitStash" => {
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_stash(working_dir)
        }
        "git_stash_pop" | "GitStashPop" => {
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_stash_pop(working_dir)
        }
        "git_add" | "GitAdd" => {
            let files = params
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_add(files, working_dir)
        }
        "git_reset" | "GitReset" => {
            let files = params
                .get("files")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(|s| s.to_string()))
                        .collect()
                })
                .unwrap_or_default();
            let working_dir = params
                .get("working_dir")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            crate::plugins::git::git_reset(files, working_dir)
        }

        // ==================== Misc Tools ====================
        "list_all" => crate::plugins::misc::tool_list_all(),

        // ==================== Browser Tools ====================
        "browser_detect" | "BrowserDetect" => {
            let browsers = crate::browser_manager::detect_chrome_installations();
            Ok(
                json!({"tool":"BrowserDetect","success":true,"browsers": browsers, "count": browsers.len()}),
            )
        }
        "browser_launch" | "BrowserLaunch" => {
            let browser_path = params
                .get("browser_path")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;
            let headless = params
                .get("headless")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            let detected = crate::browser_manager::detect_chrome_installations();
            let path = browser_path.or_else(|| detected.first().map(|b| b.path.clone()));
            let path = match path {
                Some(p) => p,
                None => return Err("No browser found. Please install Chrome or Edge.".to_string()),
            };

            let config = crate::browser_manager::ChromeLaunchConfig {
                remote_debugging_port: port,
                headless,
                ..Default::default()
            };
            let launched_port =
                crate::browser_manager::launch_chrome_with_debugging(&path, &config)?;
            Ok(
                json!({"tool":"BrowserLaunch","success":true,"port": launched_port, "browser_path": path}),
            )
        }
        "browser_navigate" | "BrowserNavigate" => {
            let url = extract_string(params, "url")?;
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            client.navigate(&url).await?;
            Ok(json!({"tool":"BrowserNavigate","success":true,"url": url}))
        }
        "browser_get_content" | "BrowserGetContent" => {
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            let content = client.get_page_content().await?;
            Ok(json!({"tool":"BrowserGetContent","success":true,"content": content}))
        }
        "browser_screenshot" | "BrowserScreenshot" => {
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            let screenshot = client.screenshot("png").await?;
            use base64::Engine;
            let b64 = base64::engine::general_purpose::STANDARD.encode(&screenshot);
            Ok(json!({"tool":"BrowserScreenshot","success":true,"image_base64": b64}))
        }
        "browser_click" | "BrowserClick" => {
            let selector = extract_string(params, "selector")?;
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            client.click_element(&selector).await?;
            Ok(json!({"tool":"BrowserClick","success":true,"selector": selector}))
        }
        "browser_fill_input" | "BrowserFillInput" => {
            let selector = extract_string(params, "selector")?;
            let value = extract_string(params, "value")?;
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            client.fill_input(&selector, &value).await?;
            Ok(json!({"tool":"BrowserFillInput","success":true,"selector": selector}))
        }
        "browser_execute_js" | "BrowserExecuteJs" => {
            let script = extract_string(params, "script")?;
            let port = params.get("port").and_then(|v| v.as_u64()).unwrap_or(9222) as u16;

            let tabs = crate::browser_manager::list_browser_tabs(port).await?;
            let tab = tabs
                .first()
                .ok_or("No browser tab found. Launch browser first.")?;
            let ws_url = tab.web_socket_url.clone();

            let client = crate::chrome_cdp::ChromeCdpClient::connect(&ws_url).await?;
            let result = client.execute_javascript(&script).await?;
            Ok(json!({"tool":"BrowserExecuteJs","success":true,"result": result}))
        }

        // ==================== UI Automation Tools ====================
        "execute_automation" | "ExecuteAutomation" => {
            let instruction = extract_string(params, "instruction")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.execute_automation(&instruction).await {
                    Ok(result) => {
                        let parsed: serde_json::Value = serde_json::from_str(&result)
                            .unwrap_or(serde_json::json!({"raw": result}));
                        let success = parsed
                            .get("success")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(true);
                        let action_count = parsed
                            .get("action_count")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(1);
                        let error_msg = parsed
                            .get("error_message")
                            .and_then(|v| v.as_str())
                            .unwrap_or("");
                        let intent = parsed.get("intent").and_then(|v| v.as_str()).unwrap_or("");

                        if success
                            && action_count <= 1
                            && (intent.starts_with("launch_app:")
                                || intent.contains("打开")
                                || intent.contains("启动"))
                        {
                            Ok(json!({
                                "tool":"ExecuteAutomation",
                                "success":true,
                                "output": format!("Application launched successfully. Intent: {}", intent),
                                "note": "Simple app launch completed. For complex multi-step tasks, provide more detailed instructions."
                            }))
                        } else if success {
                            Ok(json!({
                                "tool":"ExecuteAutomation",
                                "success":true,
                                "output": format!("Task completed in {} steps. Intent: {}", action_count, intent),
                                "steps": action_count,
                                "note": "CUA Agent completed the task. Check the result on screen."
                            }))
                        } else {
                            Ok(json!({
                                "tool":"ExecuteAutomation",
                                "success":false,
                                "output": format!("Task failed after {} steps. Intent: {}", action_count, intent),
                                "error": error_msg,
                                "steps": action_count,
                                "note": "CUA Agent could not complete the task. You may need to guide the user to perform the action manually, or try with a different instruction."
                            }))
                        }
                    }
                    Err(e) => {
                        let err_str = e.to_string();
                        let lower_inst = instruction.to_lowercase();
                        let is_launch_cmd = lower_inst.contains("打开")
                            || lower_inst.contains("启动")
                            || lower_inst.contains("launch")
                            || lower_inst.contains("open")
                            || lower_inst.contains("start")
                            || lower_inst.contains("运行")
                            || lower_inst.contains("run");

                        if is_launch_cmd {
                            log::info!(
                                "[ToolDispatcher] ExecuteAutomation failed for launch command, trying direct launch for: {}",
                                instruction
                            );

                            if let Some(launch_result) =
                                try_direct_launch_fallback(&instruction, executor).await
                            {
                                return Ok(launch_result);
                            }
                        }

                        Ok(json!({"tool":"ExecuteAutomation","success":false,"error": err_str}))
                    }
                },
                None => Err(
                    "Automation engine not initialized. Enable UI Automation in Settings > Tools."
                        .to_string(),
                ),
            }
        }
        "capture_screen" | "CaptureScreen" => match claw_traits::automation::get_executor() {
            Some(executor) => match executor.capture_screen().await {
                Ok(result) => {
                    let parsed: serde_json::Value =
                        serde_json::from_str(&result).unwrap_or(serde_json::json!({"raw": result}));
                    let ocr_summary = parsed
                        .get("ocr_summary")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    let screen_size = parsed
                        .get("screen_size")
                        .cloned()
                        .unwrap_or(serde_json::json!([0, 0]));
                    let image_base64 = parsed
                        .get("image_base64")
                        .and_then(|v| v.as_str())
                        .unwrap_or("");
                    Ok(json!({
                        "tool":"CaptureScreen",
                        "success":true,
                        "screen_size": screen_size,
                        "ocr_summary": ocr_summary,
                        "image_base64": image_base64,
                        "note": "Screen captured with OCR and image data. Use image_base64 for visual analysis, or ocr_summary for text content. Use OcrRecognizeScreen for more detailed element data with coordinates."
                    }))
                }
                Err(e) => Ok(json!({"tool":"CaptureScreen","success":false,"error": e})),
            },
            None => Err(
                "Automation engine not initialized. Enable UI Automation in Settings > Tools."
                    .to_string(),
            ),
        },
        "ocr_recognize_screen" | "OcrRecognizeScreen" => {
            let language = params
                .get("language")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.ocr_recognize_screen(language.as_deref()).await {
                    Ok(result) => {
                        Ok(json!({"tool":"OcrRecognizeScreen","success":true,"result": result}))
                    }
                    Err(e) => Ok(json!({"tool":"OcrRecognizeScreen","success":false,"error": e})),
                },
                None => Err(
                    "Automation engine not initialized. Enable UI Automation in Settings > Tools."
                        .to_string(),
                ),
            }
        }
        "mouse_click" | "MouseClick" => {
            let x = params
                .get("x")
                .and_then(|v| v.as_f64())
                .ok_or("missing x")?;
            let y = params
                .get("y")
                .and_then(|v| v.as_f64())
                .ok_or("missing y")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.mouse_click(x, y).await {
                    Ok(result) => Ok(json!({"tool":"MouseClick","success":true,"output": result})),
                    Err(e) => Ok(json!({"tool":"MouseClick","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "mouse_double_click" | "MouseDoubleClick" => {
            let x = params
                .get("x")
                .and_then(|v| v.as_f64())
                .ok_or("missing x")?;
            let y = params
                .get("y")
                .and_then(|v| v.as_f64())
                .ok_or("missing y")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.mouse_double_click(x, y).await {
                    Ok(result) => {
                        Ok(json!({"tool":"MouseDoubleClick","success":true,"output": result}))
                    }
                    Err(e) => Ok(json!({"tool":"MouseDoubleClick","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "keyboard_type" | "KeyboardType" => {
            let text = extract_string(params, "text")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.keyboard_type(&text).await {
                    Ok(result) => {
                        Ok(json!({"tool":"KeyboardType","success":true,"output": result}))
                    }
                    Err(e) => Ok(json!({"tool":"KeyboardType","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "keyboard_press" | "KeyboardPress" => {
            let key = extract_string(params, "key")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.keyboard_press(&key).await {
                    Ok(result) => {
                        Ok(json!({"tool":"KeyboardPress","success":true,"output": result}))
                    }
                    Err(e) => Ok(json!({"tool":"KeyboardPress","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "mouse_right_click" | "MouseRightClick" => {
            let x = params
                .get("x")
                .and_then(|v| v.as_f64())
                .ok_or("missing x")?;
            let y = params
                .get("y")
                .and_then(|v| v.as_f64())
                .ok_or("missing y")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.mouse_right_click(x, y).await {
                    Ok(result) => {
                        Ok(json!({"tool":"MouseRightClick","success":true,"output": result}))
                    }
                    Err(e) => Ok(json!({"tool":"MouseRightClick","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "list_installed_apps" | "ListInstalledApps" => {
            let filter = params
                .get("filter")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string());
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.list_installed_apps(filter.as_deref()).await {
                    Ok(result) => {
                        let parsed: serde_json::Value =
                            serde_json::from_str(&result).unwrap_or(json!({"raw": result}));
                        Ok(json!({"tool":"ListInstalledApps","success":true,"result": parsed}))
                    }
                    Err(e) => Ok(json!({"tool":"ListInstalledApps","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }
        "launch_application" | "LaunchApplication" => {
            let name = extract_string(params, "name")?;
            match claw_traits::automation::get_executor() {
                Some(executor) => match executor.launch_application(&name).await {
                    Ok(result) => {
                        let verified = result.contains("(active window:");
                        Ok(json!({
                            "tool":"LaunchApplication",
                            "success": verified,
                            "output": result,
                            "note": if verified { "Application launched and window detected." } else { "Launch command sent but no window change detected. The app may not be installed or may need more time." }
                        }))
                    }
                    Err(e) => Ok(json!({"tool":"LaunchApplication","success":false,"error": e})),
                },
                None => Err("Automation engine not initialized".to_string()),
            }
        }

        // ==================== Unknown Tool ====================
        _ => Err(format!(
            "未知工具: '{}'\n\n可用工具:\n\
                \n【Shell】bash, bash_cancel\n\
                【File 】file_read, file_edit, file_write\n\
                【Search】glob, grep\n\
                【Web  】web_fetch, web_search\n\
                【Agent 】agent, todo_write/get, task_create/get/update/list,\n\
                         workflow, skill, brief, config, enter/exit_plan_mode,\n\
                         notebook_edit, schedule_cron/list, ask_user_question, tool_search\n\
                【Dynamic】create_tool, create_skill\n\
                【Git  】git_status, git_diff, git_commit, git_log, git_branch,\n\
                         git_create_branch, git_checkout, git_stash, git_stash_pop,\n\
                         git_add, git_reset\n\
                【Browser】browser_detect, browser_launch, browser_navigate,\n\
                          browser_get_content, browser_screenshot, browser_click,\n\
                          browser_fill_input, browser_execute_js\n\
                【Automation】execute_automation, capture_screen, ocr_recognize_screen,\n\
                             mouse_click, mouse_double_click, mouse_right_click, keyboard_type, keyboard_press\n\
                【Misc 】list_all\n\n\
                提示: 检查工具名称是否正确",
            name
        )),
    }
}
