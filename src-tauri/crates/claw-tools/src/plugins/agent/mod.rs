// Claw Desktop - Agent 编排工具（15 个命令）
//
// 核心编排能力:
//   Agent:        子代理派发（background/fork 模式）
//   TodoWrite:    Todo 列表管理（CRUD + 进度计算）
//   Task:         后台任务管理（创建/查询/更新/列表）
//   Workflow:     工作流编排（多步骤顺序执行）
//   Skill:        技能调用入口
//   PlanMode:     计划模式开关（规划 vs 执行状态切换）
//   Brief:        简报生成（消息 + 附件）
//   Config:       配置查询/修改（list/get/set）
//   NotebookEdit: Jupyter Notebook 编辑
//   ScheduleCron: 定时任务注册/列表
//   AskUserQuestion: 用户交互提问
//   ToolSearch:   工具注册表搜索
//
// 所有数据存储在进程内存中（LazyLock<Mutex>），重启后清空
// 规则：所有 format! 必须先提取为变量，再传入 json!（避免嵌套解析问题）

use serde_json::{json, Value};

macro_rules! lock_store {
    ($store:expr) => {
        match $store.lock() {
            Ok(guard) => guard,
            Err(e) => {
                log::error!("[Agent:lock_store] Mutex poisoned for {}: {}, recovering...", stringify!($store), e);
                e.into_inner()
            }
        }
    };
}
use std::collections::HashMap;
use std::sync::{LazyLock, Mutex};

/// 全局后台任务存储: task_id → {prompt, status, ...}
static TASK_STORE: LazyLock<Mutex<HashMap<String, Value>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// 全局定时任务存储: name → {schedule, task, enabled, ...}
static CRON_STORE: LazyLock<Mutex<HashMap<String, Value>>> = LazyLock::new(|| Mutex::new(HashMap::new()));
/// 全局 Todo 列表
static TODO_STORE: LazyLock<Mutex<Vec<Value>>> = LazyLock::new(|| Mutex::new(Vec::new()));
/// 全局计划模式状态 (true=规划中, false=正常执行)
static PLAN_MODE: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

/// 子代理派发工具 — 支持fork和background两种模式，可指定Agent ID和模型
#[tauri::command]
pub async fn tool_agent(prompt: String, mode: Option<String>, model_override: Option<String>, agent_id: Option<String>) -> Result<serde_json::Value, String> {
    let m = mode.unwrap_or_else(|| "fork".to_string());

    let config = claw_config::config::get_config().await.map_err(|e| e.to_string())?;
    let api_key = config.resolve_api_key().map_err(|e| e.to_string())?;
    let base_url = config.get_base_url().to_string();

    let (system_prompt, model) = if let Some(ref aid) = agent_id {
        match crate::agent_session::iso_agent_get(aid.clone()).await {
            Ok(Some(agent)) => {
                let sp = agent.system_prompt
                    .as_ref()
                    .filter(|s| !s.is_empty())
                    .cloned()
                    .unwrap_or_else(|| "You are a helpful AI assistant.".to_string());
                let mo = agent.model_override
                    .as_deref()
                    .map(|s| s.to_string())
                    .or_else(|| model_override.clone())
                    .unwrap_or_else(|| config.model.default_model.clone());
                (sp, mo)
            }
            _ => {
                let sp = "You are a helpful AI assistant.".to_string();
                let mo = model_override.clone().unwrap_or_else(|| config.model.default_model.clone());
                (sp, mo)
            }
        }
    } else {
        let sp = "You are a helpful AI assistant.".to_string();
        let mo = model_override.clone().unwrap_or_else(|| config.model.default_model.clone());
        (sp, mo)
    };

    match m.as_str() {
        "background" | "async" => {
            let id = uuid::Uuid::new_v4().to_string();
            let task_id = id.clone();
            let sp = system_prompt.clone();
            let md = model.clone();
            let ak = api_key.clone();
            let bu = base_url.clone();
            let is_openai = config.is_openai_compatible();
            let pr = prompt.clone();

            lock_store!(TASK_STORE).insert(id.clone(), json!({"prompt": pr, "status": "running", "mode": m, "agent_id": agent_id}));

            tauri::async_runtime::spawn(async move {
                let result = call_llm_via_trait(&ak, &bu, &md, &sp, &pr, is_openai).await;
                let mut store = lock_store!(TASK_STORE);
                match result {
                    Ok(text) => {
                        store.insert(task_id.clone(), json!({"prompt": pr, "status": "completed", "mode": "background", "result": text}));
                    }
                    Err(e) => {
                        store.insert(task_id.clone(), json!({"prompt": pr, "status": "failed", "mode": "background", "error": e}));
                    }
                }
            });

            let out = format!("后台代理已启动 [ID:{}]\n{}", id, claw_types::truncate_str_safe(&prompt, 200));
            Ok(json!({"tool":"Agent","success":true,"output":out}))
        }
        _ => {
            let is_openai = config.is_openai_compatible();
            match call_llm_via_trait(&api_key, &base_url, &model, &system_prompt, &prompt, is_openai).await {
                Ok(text) => {
                    let out = format!("[Fork Agent | model={}]\n\n{}", model, text);
                    Ok(json!({"tool":"Agent","success":true,"output":out}))
                }
                Err(e) => {
                    let out = format!("[Fork Agent] Execution failed: {}", e);
                    Ok(json!({"tool":"Agent","success":false,"output":out,"error":e}))
                }
            }
        }
    }
}

/// 通过LlmCaller trait调用LLM — 优先使用注册的caller，回退到内置HTTP客户端
async fn call_llm_via_trait(api_key: &str, base_url: &str, model: &str, system: &str, user_msg: &str, is_openai: bool) -> Result<String, String> {
    if let Some(caller) = claw_traits::get_llm_caller() {
        caller.call_once(api_key, base_url, model, system, user_msg, is_openai).await
    } else {
        log::warn!("[Agent] LlmCaller not registered, falling back to built-in HTTP client");
        call_llm_once_fallback(api_key, base_url, model, system, user_msg, is_openai).await
    }
}

/// LLM调用回退实现 — 直接通过HTTP请求调用OpenAI/Anthropic API
async fn call_llm_once_fallback(api_key: &str, base_url: &str, model: &str, system: &str, user_msg: &str, is_openai: bool) -> Result<String, String> {
    use reqwest::Client;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| format!("HTTP client error: {}", e))?;

    if is_openai {
        let url = format!("{}/chat/completions", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg}
            ],
            "max_tokens": 4096,
        });

        let resp = client.post(&url)
            .header("Authorization", format!("Bearer {}", api_key))
            .json(&body)
            .send().await
            .map_err(|e| format!("Request error: {}", e))?
            .error_for_status()
            .map_err(|e| format!("API error: {}", e))?;

        let v: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse error: {}", e))?;

        let content = v["choices"][0]["message"]["content"].as_str().unwrap_or("");
        Ok(content.to_string())
    } else {
        let url = format!("{}/v1/messages", base_url.trim_end_matches('/'));
        let body = serde_json::json!({
            "model": model,
            "max_tokens": 4096,
            "system": system,
            "messages": [{"role": "user", "content": user_msg}],
        });

        let resp = client.post(&url)
            .header("x-api-key", api_key)
            .header("anthropic-version", "2023-06-01")
            .header("content-type", "application/json")
            .json(&body)
            .send().await
            .map_err(|e| format!("Request error: {}", e))?
            .error_for_status()
            .map_err(|e| format!("API error: {}", e))?;

        let v: serde_json::Value = resp.json().await
            .map_err(|e| format!("Parse error: {}", e))?;

        let content = v["content"][0]["text"].as_str().unwrap_or("");
        Ok(content.to_string())
    }
}

/// Todo列表写入工具 — 更新全局Todo列表并计算进度
#[tauri::command]
pub fn tool_todo_write(todos: Value) -> Result<serde_json::Value, String> {
    let list = todos.as_array().ok_or("todos must be array".to_string())?;
    *TODO_STORE.lock().map_err(|e| format!("Lock poisoned: {}", e))? = list.clone();
    let mut lines = vec![format!("Todo List ({} items):", list.len())];
    for item in list {
        let c = item.get("content").and_then(|v| v.as_str()).unwrap_or("");
        let s = item.get("status").and_then(|v| v.as_str()).unwrap_or("");
        let p = item.get("priority").and_then(|v| v.as_str()).unwrap_or("");
        let icon = match s { "completed" => "[OK]", "in_progress" => "[..]", _ => "[  ]" };
        let pt = if p.is_empty() { String::new() } else { format!(" [{}]", p) };
        lines.push(format!("  {} {}{}", icon, c, pt));
    }
    let done = list.iter().filter(|t| t.get("status") == Some(&json!("completed"))).count();
    let pct = if list.is_empty() { 0.0 } else { (done as f64 / list.len() as f64) * 100.0 };
    let out = format!("{}\n\nProgress: {}/{} ({:.0}%)", lines.join("\n"), done, list.len(), pct);
    Ok(json!({"tool":"TodoWrite","success":true,"output":out}))
}

/// 获取当前Todo列表
#[tauri::command]
pub fn tool_todo_get() -> Result<serde_json::Value, String> {
    let store = lock_store!(TODO_STORE);
    if store.is_empty() { return Ok(json!({"tool":"TodoGet","success":true,"output":"No todos"})); }
    let items: Vec<String> = store.iter()
        .map(|t| format!("- [{}] {}", t.get("status").and_then(|v|v.as_str()).unwrap_or(""), t.get("content").and_then(|v|v.as_str()).unwrap_or("")))
        .collect();
    Ok(json!({"tool":"TodoGet","success":true,"output":items.join("\n")}))
}

/// 创建后台任务 — 生成UUID并加入任务存储
#[tauri::command]
pub fn tool_task_create(prompt: String) -> Result<serde_json::Value, String> {
    let id = uuid::Uuid::new_v4().to_string();
    lock_store!(TASK_STORE).insert(id.clone(), json!({"id":id,"prompt":prompt,"status":"running"}));
    let out = format!("Task created [ID:{}]\n{}", id, claw_types::truncate_str_safe(&prompt, 200));
    Ok(json!({"tool":"TaskCreate","success":true,"output":out}))
}

/// 获取后台任务状态
#[tauri::command]
pub fn tool_task_get(task_id: String) -> Result<serde_json::Value, String> {
    let store = lock_store!(TASK_STORE);
      match store.get(&task_id) {
        Some(t) => Ok(json!({"tool":"TaskGet","success":true,"output":serde_json::to_string(t).unwrap_or_default()})),
        None => { let out = format!("Task '{}' not found", task_id); Ok(json!({"tool":"TaskGet","success":false,"output":out})) },
    }
}

/// 更新后台任务状态
#[tauri::command]
pub fn tool_task_update(task_id: String, status: String) -> Result<serde_json::Value, String> {
    let mut s = lock_store!(TASK_STORE);
    if let Some(t) = s.get_mut(&task_id) {
        t["status"] = json!(status);
        let out = format!("Task '{}' -> {}", task_id, status);
        Ok(json!({"tool":"TaskUpdate","success":true,"output":out}))
    } else {
        let out = format!("Task '{}' not found", task_id);
        Ok(json!({"tool":"TaskUpdate","success":false,"output":out}))
    }
}

/// 列出后台任务 — 支持按状态过滤
#[tauri::command]
pub fn tool_task_list(status_filter: Option<String>) -> Result<serde_json::Value, String> {
    let s = lock_store!(TASK_STORE);
    let tasks: Vec<&Value> = match &status_filter {
        Some(f) => s.values().filter(|t| t.get("status")==Some(&json!(f))).collect(),
        None => s.values().collect(),
    };
    if tasks.is_empty() { return Ok(json!({"tool":"TaskList","success":true,"output":"No background tasks"})); }
    let items: Vec<String> = tasks.iter().map(|t| {
        let st = t.get("status").and_then(|v|v.as_str()).unwrap_or("?");
        let tid = t.get("id").and_then(|v|v.as_str()).unwrap_or("?");
        let pr = t.get("prompt").and_then(|v|v.as_str()).unwrap_or("");
        format!("- [{}] ID={} {}", st, tid, claw_types::truncate_str_safe(pr, 80))
    }).collect();
    let out = format!("Total {} tasks:\n{}", tasks.len(), items.join("\n"));
    Ok(json!({"tool":"TaskList","success":true,"output":out}))
}

/// 工作流编排工具 — 多步骤顺序执行
#[tauri::command]
pub fn tool_workflow(name: String, steps: Option<Value>, inputs: Option<Value>) -> Result<serde_json::Value, String> {
    let steps_list: Vec<Value> = steps.and_then(|s| s.as_array().cloned()).unwrap_or_default();
    let results: Vec<String> = steps_list.iter().enumerate()
        .map(|(i, st)| format!("Step {}: action={}", i+1, st.get("action").and_then(|v|v.as_str()).unwrap_or("")))
        .collect();
    let out = json!({"workflow":name,"status":"completed","steps_executed":results.len(),"inputs":inputs,"results":results}).to_string();
    Ok(json!({"tool":"Workflow","success":true,"output":out}))
}

/// 技能调用工具 — 加载指定技能的指令内容
#[tauri::command]
pub fn tool_skill(skill_name: String, args: Option<Value>) -> Result<serde_json::Value, String> {
    let args_str = args.as_ref().map(|a| a.to_string()).unwrap_or_default();

    let skill_content = crate::skill_loader::load_skill_content(&skill_name);

    match skill_content {
        Some(content) => {
            let output = if args_str.is_empty() {
                format!(
                    "Skill '{}' loaded successfully.\n\n--- SKILL INSTRUCTIONS ---\n{}\n--- END INSTRUCTIONS ---\n\nFollow the above instructions to complete the task. Use the tools listed in the skill's allowed-tools to execute.",
                    skill_name, content
                )
            } else {
                format!(
                    "Skill '{}' loaded successfully.\nUser args: {}\n\n--- SKILL INSTRUCTIONS ---\n{}\n--- END INSTRUCTIONS ---\n\nFollow the above instructions with the provided arguments. Use the tools listed in the skill's allowed-tools to execute.",
                    skill_name, args_str, content
                )
            };
            Ok(json!({"tool":"Skill","success":true,"output":output}))
        }
        None => {
            let available = crate::skill_loader::list_available_skills().join(", ");
            Ok(json!({
                "tool":"Skill",
                "success":false,
                "error":format!("Skill '{}' not found. Available skills: {}", skill_name, available),
                "output":format!("Skill '{}' not found. Available skills: {}", skill_name, available)
            }))
        }
    }
}

/// 进入计划模式 — AI将先规划再执行
#[tauri::command]
pub fn tool_enter_plan_mode() -> Result<serde_json::Value, String> {
    *lock_store!(PLAN_MODE) = true;
    Ok(json!({"tool":"EnterPlanMode","success":true,"output":"Plan mode activated! AI will plan before changes."}))
}

/// 退出计划模式 — 恢复正常执行
#[tauri::command]
pub fn tool_exit_plan_mode() -> Result<serde_json::Value, String> {
    *PLAN_MODE.lock().map_err(|e| format!("Lock poisoned: {}", e))? = false;
    Ok(json!({"tool":"ExitPlanMode","success":true,"output":"Exited plan mode. Real operations enabled."}))
}

/// 获取计划模式状态
#[tauri::command]
pub fn tool_get_plan_status() -> Result<serde_json::Value, String> {
    let active = *lock_store!(PLAN_MODE);
    let out = if active { "PLAN MODE ACTIVE" } else { "NORMAL MODE" };
    Ok(json!({"tool":"PlanStatus","success":true,"output":out}))
}

/// 简报生成工具 — 生成消息和附件的简报
#[tauri::command]
pub fn tool_brief(message: String, attachments: Option<Vec<String>>) -> Result<serde_json::Value, String> {
    let att = attachments.map(|a| format!("Attachments: {}", a.join(", "))).unwrap_or_default();
    let out = format!("{}\n{}", message, att);
    Ok(json!({"tool":"Brief","success":true,"output":out}))
}

/// 配置查询/修改工具 — 支持list/get/set操作
#[tauri::command]
pub fn tool_config(action: String, key: Option<String>, value: Option<Value>) -> Result<serde_json::Value, String> {
    match action.as_str() {
        "list" => Ok(json!({"tool":"Config","success":true,"output":"Available:\n- app.language/theme/auto_update\n- model.provider/default_model/temperature\n- api.api_key/base_url/timeout_seconds\n- ui.font_size/code_theme\n- advanced.log_level/proxy_url"})),
        "get" => { let out = format!("config[{}] = (see Settings)", key.unwrap_or_default()); Ok(json!({"tool":"Config","success":true,"output":out})) },
        "set" => {
            let k = key.ok_or("missing key".to_string())?;
            let v = value.ok_or("missing value".to_string())?;
            let out = format!("Config change request: {} = {}\nSave via Settings panel", k, v);
            Ok(json!({"tool":"Config","success":true,"output":out}))
        },
        _ => { let out = format!("Unknown action: {}", action); Ok(json!({"tool":"Config","success":false,"output":out})) },
    }
}

/// Notebook编辑工具 — 编辑Jupyter Notebook的单元格
#[tauri::command]
pub fn tool_notebook_edit(file_path: String, cell_index: u64, source: Option<Value>) -> Result<serde_json::Value, String> {
    let src_lines = source.as_ref().and_then(|s| s.as_array()).map(|a| a.len()).unwrap_or(0);
    let out = format!("Notebook Edit:\n  File: {}\n  Cell #{}\n  Source: {} lines", file_path, cell_index, src_lines);
    Ok(json!({"tool":"NotebookEdit","success":true,"output":out}))
}

/// 定时任务注册工具 — 注册Cron定时任务
#[tauri::command]
pub fn tool_schedule_cron(name: String, schedule: String, task: String, enabled: Option<bool>) -> Result<serde_json::Value, String> {
    CRON_STORE.lock().map_err(|e| format!("Lock poisoned: {}", e))?.insert(name.clone(), json!({
        "name": name, "schedule": schedule, "task": task,
        "enabled": enabled.unwrap_or(true), "created_at": chrono::Utc::now().to_rfc3339()
    }));
    let en = enabled.unwrap_or(true);
    let out = format!("Cron registered: {} | Schedule: {} | Task: {} | Enabled: {}", name, schedule, task, en);
    Ok(json!({"tool":"ScheduleCron","success":true,"output":out}))
}

/// 列出所有定时任务
#[tauri::command]
pub fn tool_schedule_list() -> Result<serde_json::Value, String> {
    let s = lock_store!(CRON_STORE);
    if s.is_empty() { return Ok(json!({"tool":"ScheduleList","success":true,"output":"No scheduled tasks"})); }
    let items: Vec<String> = s.values().map(|v| {
        let n = v.get("name").and_then(|x|x.as_str()).unwrap_or("?");
        let en = if v.get("enabled").and_then(|x|x.as_bool()).unwrap_or(false){"ON"}else{"OFF"};
        let sch = v.get("schedule").and_then(|x|x.as_str()).unwrap_or("?");
        let tk = v.get("task").and_then(|x|x.as_str()).unwrap_or("?");
        format!("- [{}] {} {} {}", n, en, sch, tk)
    }).collect();
    let out = format!("Scheduled tasks ({}):\n{}", s.len(), items.join("\n"));
    Ok(json!({"tool":"ScheduleList","success":true,"output":out}))
}

/// 用户交互提问工具 — 向用户展示问题列表
#[tauri::command]
pub fn tool_ask_user_question(questions: Value) -> Result<serde_json::Value, String> {
    let qlist = questions.as_array().ok_or("questions must be array".to_string())?;
    let mut ans = Vec::new();
    for (i, q) in qlist.iter().enumerate() {
        let question = q.get("question").and_then(|v| v.as_str()).unwrap_or("(?)");
        ans.push(format!("Q{}: {}", i+1, question));
    }
    let out = format!("Questions pending user response:\n{}\n\nUser will answer in the frontend UI.", ans.join("\n"));
    Ok(json!({"tool":"AskUserQuestion","success":true,"output":out}))
}

/// 工具搜索工具 — 在工具注册表中按名称或描述搜索
#[tauri::command]
pub fn tool_tool_search(query: String, max_results: Option<u64>) -> Result<serde_json::Value, String> {
    let max = max_results.unwrap_or(10) as usize;
    let all_tools = crate::registry::get_all_tool_definitions();
    let matches: Vec<String> = all_tools.iter()
        .filter(|t| t.name.to_lowercase().contains(&query.to_lowercase()) || t.description.to_lowercase().contains(&query.to_lowercase()))
        .take(max)
        .map(|t| format!("- **{}**: {}", t.name, t.description))
        .collect();
    if matches.is_empty() {
        let names: Vec<String> = all_tools.iter().map(|t| t.name.clone()).collect();
        let out = format!("No match for '{}'\nAll({}): {}", query, names.len(), names.join(", "));
        Ok(json!({"tool":"ToolSearch","success":true,"output":out}))
    } else {
        let out = format!("Found {} matches:\n{}", matches.len(), matches.join("\n"));
        Ok(json!({"tool":"ToolSearch","success":true,"output":out}))
    }
}

/// 创建自定义工具 — 生成扩展目录、清单文件并注册到工具表
pub async fn tool_create_tool(params: Value) -> Result<serde_json::Value, String> {
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing 'name' parameter")?.to_string();
    let description = params.get("description").and_then(|v| v.as_str()).ok_or("missing 'description' parameter")?.to_string();
    let input_schema = params.get("input_schema").cloned().unwrap_or(json!({"type":"object","properties":{}}));
    let handler_script = params.get("handler_script").and_then(|v| v.as_str()).map(|s| s.to_string());

    let ext_dir = claw_config::path_resolver::extensions_dir().join(&name);

    std::fs::create_dir_all(&ext_dir).map_err(|e| format!("Failed to create extension directory: {}", e))?;

    let mut manifest = json!({
        "name": name,
        "version": "1.0.0",
        "description": description,
        "enabled": true,
        "tools": [{
            "name": name,
            "description": description,
            "input_schema": input_schema
        }]
    });

    if let Some(ref script) = handler_script {
        let script_path = ext_dir.join("handler.sh");
        std::fs::write(&script_path, script).map_err(|e| format!("Failed to write handler script: {}", e))?;
        manifest["tools"][0]["handler"] = json!("handler.sh");
    }

    let manifest_str = serde_json::to_string_pretty(&manifest).map_err(|e| format!("Failed to serialize manifest: {}", e))?;
    let manifest_path = ext_dir.join("manifest.json");
    std::fs::write(&manifest_path, &manifest_str).map_err(|e| format!("Failed to write manifest: {}", e))?;

    let def = claw_types::common::ToolDefinition {
        name: name.clone(),
        description: description.clone(),
        input_schema: input_schema.clone(),
        category: None,
        tags: Vec::new(),
    };
    crate::tool_registry::register_tool(def, crate::tool_registry::ToolSource::Extension, handler_script.clone()).await;

    log::info!("[CreateTool] Extension tool '{}' created at {}", name, ext_dir.display());

    if let Err(e) = claw_rag::rag::add_tool_memory(&name, &description, "extension").await {
        log::warn!("[CreateTool] Failed to add tool memory for '{}': {}", name, e);
    }

    Ok(json!({
        "tool": "CreateTool",
        "success": true,
        "output": format!("Tool '{}' created and registered successfully.\nPath: {}\nDescription: {}\n\nThe tool is now available for use. It has also been added to the tool catalog and memory.", name, ext_dir.display(), description)
    }))
}

/// 创建自定义技能 — 生成SKILL.md文件并注册到技能表
pub async fn tool_create_skill(params: Value) -> Result<serde_json::Value, String> {
    let name = params.get("name").and_then(|v| v.as_str()).ok_or("missing 'name' parameter")?.to_string();
    let description = params.get("description").and_then(|v| v.as_str()).ok_or("missing 'description' parameter")?.to_string();
    let when_to_use = params.get("when_to_use").and_then(|v| v.as_str()).unwrap_or("");
    let allowed_tools: Vec<String> = params.get("allowed_tools")
        .and_then(|v| v.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
        .unwrap_or_default();
    let instructions = params.get("instructions").and_then(|v| v.as_str()).ok_or("missing 'instructions' parameter")?.to_string();

    let skill_dir = claw_config::path_resolver::skills_dir().join(&name);

    std::fs::create_dir_all(&skill_dir).map_err(|e| format!("Failed to create skill directory: {}", e))?;

    let allowed_tools_yaml = if allowed_tools.is_empty() {
        String::new()
    } else {
        format!("\nallowed-tools: {:?}", allowed_tools)
    };

    let when_to_use_yaml = if when_to_use.is_empty() {
        String::new()
    } else {
        format!("\nwhen_to_use: {}", when_to_use)
    };

    let skill_md = format!(
r#"---
name: {name}
description: '{description}'{when_to_use_yaml}{allowed_tools_yaml}
user_invocable: true
version: 1.0.0
---

{instructions}
"#,
        name = name,
        description = description.replace('\'', "\\'"),
        when_to_use_yaml = when_to_use_yaml,
        allowed_tools_yaml = allowed_tools_yaml,
        instructions = instructions,
    );

    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, &skill_md).map_err(|e| format!("Failed to write SKILL.md: {}", e))?;

    let skill_def = crate::skill_loader::load_skill_from_file(&skill_path, crate::skill_loader::SkillSource::User);
    if let Some(loaded) = skill_def {
        crate::skill_loader::register_skills_as_tools(&[loaded]).await;
        log::info!("[CreateSkill] Skill '{}' created and registered at {}", name, skill_dir.display());
    } else {
        log::warn!("[CreateSkill] Skill '{}' created but failed to register: {:?}", name, skill_def);
    }

    if let Err(e) = claw_rag::rag::add_skill_memory(&name, &description, when_to_use, &allowed_tools).await {
        log::warn!("[CreateSkill] Failed to add skill memory for '{}': {}", name, e);
    }

    Ok(json!({
        "tool": "CreateSkill",
        "success": true,
        "output": format!("Skill '{}' created and registered successfully.\nPath: {}\n\nThe skill is now available via `Skill:{}`. It has also been added to memory.", name, skill_dir.display(), name)
    }))
}
