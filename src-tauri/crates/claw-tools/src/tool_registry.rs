// Claw Desktop - 工具注册表 - 管理所有可用工具的注册和查询
// 对标 def_claw tools.ts + pluginLoader 的动态注册能力
// 使用 tokio::sync::RwLock 实现异步安全的全局工具注册表

use claw_types::common::ToolDefinition;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};
use serde_json::{self, Value, json};
use std::collections::HashMap;
use std::sync::OnceLock;
use tokio::sync::RwLock;

/// 工具来源枚举 - 标识工具的注册来源
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum ToolSource {
    BuiltIn,   // 内置工具
    Skill,     // 技能工具
    Extension, // 扩展工具
    Mcp,       // MCP协议工具
}

impl Default for ToolSource {
    fn default() -> Self {
        Self::BuiltIn
    }
}

/// 已注册工具条目 - 包含工具定义、处理器名称和来源
struct RegisteredTool {
    definition: ToolDefinition, // 工具定义
    #[allow(dead_code)]
    handler_name: Option<String>, // 处理器函数名
    source: ToolSource,         // 工具来源
}

/// 全局工具注册表（tokio::sync::RwLock，支持 async 读写）
static TOOL_REGISTRY: OnceLock<RwLock<ToolRegistryInner>> = OnceLock::new();

/// 工具注册表内部结构 - 管理所有已注册工具的HashMap
struct ToolRegistryInner {
    tools: HashMap<String, RegisteredTool>, // 工具名→工具条目的映射
}

impl ToolRegistryInner {
    /// 创建新的空注册表
    fn new() -> Self {
        Self {
            tools: HashMap::new(),
        }
    }

    /// 注册工具到注册表，返回是否成功
    fn register(
        &mut self,
        def: ToolDefinition,
        source: ToolSource,
        handler: Option<String>,
    ) -> bool {
        let name = def.name.clone();
        self.tools.insert(
            name.clone(),
            RegisteredTool {
                definition: def,
                handler_name: handler,
                source,
            },
        );
        log::info!(
            "[ToolRegistry] 注册工具: {} (来源: {:?})",
            name,
            self.tools
                .get(&name)
                .map(|r| &r.source)
                .unwrap_or(&ToolSource::BuiltIn)
        );
        true
    }

    /// 取消注册工具，返回被移除的工具定义
    fn unregister(&mut self, name: &str) -> Option<ToolDefinition> {
        self.tools.remove(name).map(|r| r.definition)
    }

    /// 获取工具定义
    #[allow(dead_code)]
    fn get(&self, name: &str) -> Option<&ToolDefinition> {
        self.tools.get(name).map(|r| &r.definition)
    }

    /// 获取工具处理器名称
    #[allow(dead_code)]
    fn get_handler(&self, name: &str) -> Option<String> {
        self.tools.get(name).and_then(|r| r.handler_name.clone())
    }

    /// 获取工具来源
    #[allow(dead_code)]
    fn get_source(&self, name: &str) -> Option<ToolSource> {
        self.tools.get(name).map(|r| r.source.clone())
    }

    /// 列出所有已注册工具及其来源
    fn list_all(&self) -> Vec<(ToolDefinition, ToolSource)> {
        self.tools
            .values()
            .map(|r| (r.definition.clone(), r.source.clone()))
            .collect()
    }

    /// 按来源筛选工具列表
    fn list_by_source(&self, src: &ToolSource) -> Vec<ToolDefinition> {
        self.tools
            .values()
            .filter(|r| &r.source == src)
            .map(|r| r.definition.clone())
            .collect()
    }

    /// 获取已注册工具总数
    fn count(&self) -> usize {
        self.tools.len()
    }

    /// 清除所有动态注册的工具（保留内置工具），返回清除数量
    #[allow(dead_code)]
    fn clear_dynamic(&mut self) -> usize {
        let before = self.tools.len();
        self.tools
            .retain(|_, t| matches!(t.source, ToolSource::BuiltIn));
        before - self.tools.len()
    }
}

/// 获取全局注册表的写锁引用（首次访问时自动初始化）
async fn registry() -> &'static RwLock<ToolRegistryInner> {
    TOOL_REGISTRY.get_or_init(|| RwLock::new(ToolRegistryInner::new()))
}

/// 注册工具到全局注册表（async，使用 tokio RwLock 写锁）
pub async fn register_tool(
    def: ToolDefinition,
    source: ToolSource,
    handler: Option<String>,
) -> bool {
    let reg = registry().await;
    let mut w = reg.write().await;
    w.register(def, source, handler)
}

/// 注销工具（async）
pub async fn unregister_tool(name: &str) -> Option<ToolDefinition> {
    let reg = registry().await;
    let mut w = reg.write().await;
    w.unregister(name)
}

/// 获取工具定义
#[allow(dead_code)]
pub async fn get_tool(name: &str) -> Option<ToolDefinition> {
    let reg = registry().await;
    let r = reg.read().await;
    r.get(name).cloned()
}
/// 获取工具处理器名称
#[allow(dead_code)]
pub async fn get_tool_handler(name: &str) -> Option<String> {
    let reg = registry().await;
    let r = reg.read().await;
    r.get_handler(name)
}
/// 获取已注册工具总数
pub async fn tool_count() -> usize {
    let reg = registry().await;
    let r = reg.read().await;
    r.count()
}
/// 清除所有动态注册的工具（保留内置工具）
#[allow(dead_code)]
pub async fn clear_dynamic_tools() -> usize {
    let reg = registry().await;
    let mut w = reg.write().await;
    w.clear_dynamic()
}

/// 列出所有工具 — 根据全局工具设置过滤
pub async fn list_all_tools() -> Vec<ToolDefinition> {
    let reg = registry().await;
    let r = reg.read().await;
    let all_tools: Vec<ToolDefinition> = r.list_all().into_iter().map(|(d, _)| d).collect();

    let config = match claw_config::config::get_config().await {
        Ok(c) => c.clone(),
        Err(_) => claw_config::config::AppConfig::default(),
    };
    let tool_settings = config.tools;
    filter_tools_by_settings(all_tools, tool_settings)
}

/// 按工具设置过滤工具列表 — 根据ToolSettings中各类开关过滤
fn filter_tools_by_settings(
    tools: Vec<ToolDefinition>,
    settings: claw_config::config::ToolSettings,
) -> Vec<ToolDefinition> {
    let file_tools = [
        "Read",
        "Edit",
        "Write",
        "file_read",
        "file_edit",
        "file_write",
    ];
    let file_write_tools = ["Edit", "Write", "file_edit", "file_write"];
    let shell_tools = ["Bash", "bash", "bash_cancel"];
    let search_tools = ["Glob", "Grep", "glob", "grep"];
    let web_tools = [
        "WebFetch",
        "WebSearch",
        "web_fetch",
        "web_search",
        "fetch",
        "search",
    ];
    let git_tools = [
        "GitStatus",
        "GitDiff",
        "GitCommit",
        "GitLog",
        "GitBranch",
        "GitCheckout",
        "GitStash",
        "GitAdd",
        "GitReset",
        "git_status",
        "git_diff",
        "git_commit",
        "git_log",
        "git_branch",
        "git_checkout",
        "git_stash",
        "git_add",
        "git_reset",
        "git_create_branch",
        "git_stash_pop",
    ];
    let browser_tools = [
        "BrowserDetect",
        "BrowserLaunch",
        "BrowserNavigate",
        "BrowserGetContent",
        "BrowserScreenshot",
        "BrowserClick",
        "BrowserFillInput",
        "BrowserExecuteJs",
        "browser_detect",
        "browser_launch",
        "browser_navigate",
        "browser_get_content",
        "browser_screenshot",
        "browser_click",
        "browser_fill_input",
        "browser_execute_js",
    ];
    let automation_tools = [
        "ExecuteAutomation",
        "CaptureScreen",
        "OcrRecognizeScreen",
        "MouseClick",
        "MouseDoubleClick",
        "MouseRightClick",
        "KeyboardType",
        "KeyboardPress",
        "ListInstalledApps",
        "LaunchApplication",
        "execute_automation",
        "capture_screen",
        "ocr_recognize_screen",
        "mouse_click",
        "mouse_double_click",
        "mouse_right_click",
        "keyboard_type",
        "keyboard_press",
        "list_installed_apps",
        "launch_application",
    ];
    let agent_tools = [
        "Agent",
        "TodoWrite",
        "TaskCreate",
        "TaskList",
        "Workflow",
        "Skill",
        "EnterPlanMode",
        "ExitPlanMode",
        "Brief",
        "Config",
        "NotebookEdit",
        "ScheduleCron",
        "AskUserQuestion",
        "ToolSearch",
        "agent",
        "todo_write",
        "todo_get",
        "task_create",
        "task_get",
        "task_update",
        "task_list",
        "workflow",
        "skill",
        "enter_plan_mode",
        "exit_plan_mode",
        "brief",
        "config",
        "notebook_edit",
        "schedule_cron",
        "schedule_list",
        "ask_user_question",
        "tool_search",
    ];

    tools
        .into_iter()
        .filter(|t| {
            let name = t.name.as_str();
            if file_tools.contains(&name) && !settings.file_access {
                return false;
            }
            if file_write_tools.contains(&name) && !settings.file_write {
                return false;
            }
            if shell_tools.contains(&name) && !settings.shell {
                return false;
            }
            if search_tools.contains(&name) && !settings.search {
                return false;
            }
            if web_tools.contains(&name) && !settings.web {
                return false;
            }
            if git_tools.contains(&name) && !settings.git {
                return false;
            }
            if browser_tools.contains(&name) && !settings.browser {
                return false;
            }
            if automation_tools.contains(&name) && !settings.automation {
                return false;
            }
            if agent_tools.contains(&name) && !settings.agent {
                return false;
            }
            true
        })
        .collect()
}

/// 列出指定Agent可用的工具 — 根据Agent的skills_enabled配置过滤
pub async fn list_tools_for_agent(agent_id: &str) -> Vec<ToolDefinition> {
    let all_tools: Vec<ToolDefinition> = list_all_tools().await;

    if let Some(agent_db) = claw_db::db::try_get_agent_db() {
        if let Ok(Some(agent)) =
            claw_db::db::agent_entities::agents::Entity::find_by_id(agent_id.to_string())
                .one(agent_db)
                .await
        {
            let enabled: Vec<String> = if let Some(ref skills_str) = agent.skills_enabled {
                serde_json::from_str(skills_str.as_str()).unwrap_or_default()
            } else {
                Vec::new()
            };
            if !enabled.is_empty() {
                let total_count = all_tools.len();
                let mut tool_names: std::collections::HashSet<String> =
                    std::collections::HashSet::new();
                for skill_id in &enabled {
                    match skill_id.as_str() {
                        "file_read" => {
                            tool_names.extend(["Read", "file_read"].map(String::from));
                        }
                        "file_write" => {
                            tool_names.extend(["Write", "file_write"].map(String::from));
                        }
                        "file_edit" => {
                            tool_names.extend(["Edit", "file_edit"].map(String::from));
                        }
                        "web_search" => {
                            tool_names.extend(
                                ["WebSearch", "WebFetch", "web_search", "web_fetch"]
                                    .map(String::from),
                            );
                        }
                        "code_exec" => {
                            tool_names.extend(["Bash", "bash", "bash_cancel"].map(String::from));
                        }
                        "git_ops" => {
                            tool_names.extend(
                                [
                                    "GitStatus",
                                    "GitDiff",
                                    "GitCommit",
                                    "GitLog",
                                    "GitBranch",
                                    "GitCheckout",
                                    "GitStash",
                                    "GitAdd",
                                    "GitReset",
                                    "git_status",
                                    "git_diff",
                                    "git_commit",
                                    "git_log",
                                    "git_branch",
                                    "git_checkout",
                                    "git_stash",
                                    "git_add",
                                    "git_reset",
                                    "git_create_branch",
                                    "git_stash_pop",
                                ]
                                .map(String::from),
                            );
                        }
                        "desktop_automation" => {
                            tool_names.extend(
                                [
                                    "ExecuteAutomation",
                                    "CaptureScreen",
                                    "OcrRecognizeScreen",
                                    "MouseClick",
                                    "MouseDoubleClick",
                                    "MouseRightClick",
                                    "KeyboardType",
                                    "KeyboardPress",
                                    "ListInstalledApps",
                                    "LaunchApplication",
                                    "execute_automation",
                                    "capture_screen",
                                    "ocr_recognize_screen",
                                    "mouse_click",
                                    "mouse_double_click",
                                    "mouse_right_click",
                                    "keyboard_type",
                                    "keyboard_press",
                                    "list_installed_apps",
                                    "launch_application",
                                ]
                                .map(String::from),
                            );
                        }
                        _ => {
                            tool_names.insert(skill_id.clone());
                        }
                    }
                }
                let filtered: Vec<ToolDefinition> = all_tools
                    .into_iter()
                    .filter(|t| tool_names.contains(&t.name))
                    .collect();
                log::info!(
                    "[ToolRegistry] Agent={} filtered tools: {}/{}",
                    claw_types::truncate_str_safe(agent_id, 12),
                    filtered.len(),
                    total_count
                );
                return filtered;
            }
        }
    }

    all_tools
}

/// 列出所有工具及其来源信息
pub async fn list_all_tools_with_source() -> Vec<Value> {
    let reg = registry().await;
    let r = reg.read().await;
    r.list_all()
        .into_iter()
        .map(|(d, s)| {
            json!({
                "name": d.name,
                "description": d.description,
                "source": format!("{:?}", s),
                "input_schema": d.input_schema,
            })
        })
        .collect()
}

/// 列出所有内置工具
#[allow(dead_code)]
pub async fn list_builtin_tools() -> Vec<ToolDefinition> {
    let reg = registry().await;
    let r = reg.read().await;
    r.list_by_source(&ToolSource::BuiltIn)
}

/// 列出所有动态工具（Skill/Extension/MCP来源）
pub async fn list_dynamic_tools() -> Vec<ToolDefinition> {
    let reg = registry().await;
    let r = reg.read().await;
    let mut result = r.list_by_source(&ToolSource::Skill);
    result.extend(r.list_by_source(&ToolSource::Extension));
    result.extend(r.list_by_source(&ToolSource::Mcp));
    result
}

/// 初始化内置工具（在应用启动时调用一次）
pub async fn init_builtin_tools() {
    let builtins = crate::registry::get_all_tool_definitions();
    let reg = registry().await;
    let mut w = reg.write().await;
    let mut count: usize = 0;
    for def in &builtins {
        w.register(def.clone(), ToolSource::BuiltIn, Some(def.name.clone()));
        count += 1;
    }
    log::info!("[ToolRegistry] 初始化完成，已注册 {} 个内置工具", count);
}

/// Tauri命令：列出所有工具
#[tauri::command]
pub async fn cmd_list_all_tools() -> Result<Value, String> {
    Ok(json!({"total": tool_count().await, "tools": list_all_tools_with_source().await}))
}

/// Tauri命令：注册新工具
#[tauri::command]
pub async fn cmd_register_tool(
    name: String,
    description: String,
    input_schema: Value,
    handler: Option<String>,
) -> Result<Value, String> {
    let def = ToolDefinition {
        name: name.clone(),
        description,
        input_schema,
        category: None,
        tags: Vec::new(),
    };
    if register_tool(def, ToolSource::Extension, handler).await {
        Ok(json!({"success": true, "message": format!("Tool '{}' registered", name)}))
    } else {
        Err(format!("Registration failed: {}", name))
    }
}

/// Tauri命令：注销工具
#[tauri::command]
pub async fn cmd_unregister_tool(name: String) -> Result<Value, String> {
    match unregister_tool(&name).await {
        Some(_) => Ok(json!({"success": true, "message": format!("Tool '{}' unregistered", name)})),
        None => Err(format!("Tool '{}' not found", name)),
    }
}
