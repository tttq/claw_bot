// Claw Desktop - TypeScript类型 - 生成前端TypeScript类型定义
use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// 前端应用配置 — 包含应用、模型、API、UI、高级和工具设置
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsAppConfig {
    pub app: TsAppSettings,
    pub model: TsModelSettings,
    pub api: TsApiSettings,
    pub ui: TsUiSettings,
    pub advanced: TsAdvancedSettings,
    pub harness: TsHarnessSettings,
    #[serde(default)]
    pub tools: TsToolSettings,
}

/// 应用基础设置 — 语言、主题、自动更新和启动行为
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsAppSettings {
    pub language: String,
    pub theme: String,
    #[serde(default)]
    pub auto_update: bool,
    #[serde(default)]
    pub minimize_to_tray: bool,
    pub startup_behavior: String,
}

/// 模型设置 — 默认模型、提供商、温度、Token限制和流式模式
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsModelSettings {
    pub default_model: String,
    pub provider: String,
    #[serde(default)]
    pub custom_url: String,
    #[serde(default)]
    pub custom_api_key: String,
    #[serde(default)]
    pub custom_model_name: String,
    pub temperature: f64,
    pub max_tokens: u32,
    pub top_p: f64,
    #[serde(default)]
    pub thinking_budget: u64,
    #[serde(default = "default_true")]
    pub stream_mode: bool,
    pub api_format: String,
}

/// API设置 — 密钥、基础URL、版本、超时和重试次数
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsApiSettings {
    #[serde(default)]
    pub api_key: String,
    pub base_url: String,
    pub api_version: String,
    pub timeout_seconds: u64,
    pub retry_count: u32,
}

/// UI设置 — 字体、侧边栏宽度、代码主题和消息样式
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsUiSettings {
    pub font_size: u32,
    pub font_family: String,
    pub sidebar_width: u32,
    #[serde(default)]
    pub show_line_numbers: bool,
    pub code_theme: String,
    pub message_style: String,
    #[serde(default = "default_true")]
    pub show_tool_executions: bool,
}

/// 高级设置 — 数据目录、日志级别、历史记录限制和代理
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsAdvancedSettings {
    #[serde(default)]
    pub data_dir: String,
    pub log_level: String,
    pub max_conversation_history: u32,
    pub auto_compact_tokens: u64,
    #[serde(default)]
    pub proxy_url: String,
    #[serde(default = "default_true")]
    pub enable_telemetry: bool,
}

/// Harness设置 — 错误学习、跨记忆、验证、并行子任务和超时
#[derive(Debug, Clone, Serialize, Deserialize, Default, TS)]
#[ts(export)]
pub struct TsHarnessSettings {
    #[serde(default)]
    pub error_learning_enabled: bool,
    #[serde(default = "default_true")]
    pub cross_memory_enabled: bool,
    #[serde(default = "default_true")]
    pub validation_enabled: bool,
    #[serde(default = "default_visibility")]
    pub default_memory_visibility: String,
    #[serde(default = "default_parallel")]
    pub max_parallel_subtasks: usize,
    #[serde(default = "default_task_timeout")]
    pub task_timeout_seconds: u64,
    #[serde(default = "default_true")]
    pub agents_md_auto_refresh: bool,
}

/// 工具权限设置 — 控制各类工具的启用/禁用状态
#[derive(Debug, Clone, Serialize, Deserialize, TS, Default)]
#[ts(export)]
pub struct TsToolSettings {
    #[serde(default = "default_true")]
    pub file_access: bool,
    #[serde(default = "default_true")]
    pub file_write: bool,
    #[serde(default = "default_true")]
    pub shell: bool,
    #[serde(default = "default_true")]
    pub search: bool,
    #[serde(default = "default_true")]
    pub web: bool,
    #[serde(default = "default_true")]
    pub git: bool,
    #[serde(default = "default_true")]
    pub browser: bool,
    #[serde(default)]
    pub automation: bool,
    #[serde(default = "default_true")]
    pub agent: bool,
}

fn default_visibility() -> String {
    "public".to_string()
}
fn default_parallel() -> usize {
    5
}
fn default_task_timeout() -> u64 {
    300
}
fn default_true() -> bool {
    true
}

impl From<&crate::config::AppConfig> for TsAppConfig {
    fn from(c: &crate::config::AppConfig) -> Self {
        Self {
            app: TsAppSettings {
                language: c.app.language.clone(),
                theme: c.app.theme.clone(),
                auto_update: c.app.auto_update,
                minimize_to_tray: c.app.minimize_to_tray,
                startup_behavior: c.app.startup_behavior.clone(),
            },
            model: TsModelSettings {
                default_model: c.model.default_model.clone(),
                provider: c.model.provider.clone(),
                custom_url: c.model.custom_url.clone(),
                custom_api_key: c.model.custom_api_key.clone(),
                custom_model_name: c.model.custom_model_name.clone(),
                temperature: c.model.temperature,
                max_tokens: c.model.max_tokens,
                top_p: c.model.top_p,
                thinking_budget: c.model.thinking_budget,
                stream_mode: c.model.stream_mode,
                api_format: c.model.api_format.clone(),
            },
            api: TsApiSettings {
                api_key: c.api.api_key.clone(),
                base_url: c.api.base_url.clone(),
                api_version: c.api.api_version.clone(),
                timeout_seconds: c.api.timeout_seconds,
                retry_count: c.api.retry_count,
            },
            ui: TsUiSettings {
                font_size: c.ui.font_size,
                font_family: c.ui.font_family.clone(),
                sidebar_width: c.ui.sidebar_width,
                show_line_numbers: c.ui.show_line_numbers,
                code_theme: c.ui.code_theme.clone(),
                message_style: c.ui.message_style.clone(),
                show_tool_executions: c.ui.show_tool_executions,
            },
            advanced: TsAdvancedSettings {
                data_dir: c.advanced.data_dir.clone(),
                log_level: c.advanced.log_level.clone(),
                max_conversation_history: c.advanced.max_conversation_history,
                auto_compact_tokens: c.advanced.auto_compact_tokens,
                proxy_url: c.advanced.proxy_url.clone(),
                enable_telemetry: c.advanced.enable_telemetry,
            },
            harness: TsHarnessSettings {
                error_learning_enabled: c.harness.error_learning_enabled,
                cross_memory_enabled: c.harness.cross_memory_enabled,
                validation_enabled: c.harness.validation_enabled,
                default_memory_visibility: c.harness.default_memory_visibility.clone(),
                max_parallel_subtasks: c.harness.max_parallel_subtasks,
                task_timeout_seconds: c.harness.task_timeout_seconds,
                agents_md_auto_refresh: c.harness.agents_md_auto_refresh,
            },
            tools: TsToolSettings {
                file_access: c.tools.file_access,
                file_write: c.tools.file_write,
                shell: c.tools.shell,
                search: c.tools.search,
                web: c.tools.web,
                git: c.tools.git,
                browser: c.tools.browser,
                automation: c.tools.automation,
                agent: c.tools.agent,
            },
        }
    }
}

/// 聊天响应 — 包含文本、用量、工具调用和工具执行信息
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsChatResponse {
    pub text: String,
    pub usage: Option<TsUsageInfo>,
    pub tool_calls: Vec<TsToolCallInfo>,
    pub tool_executions: Vec<TsToolExecutionInfo>,
}

/// Token用量信息 — 输入/输出Token数及缓存统计
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsUsageInfo {
    pub input_tokens: u64,
    pub output_tokens: u64,
    pub cache_read_tokens: Option<u64>,
    pub cache_creation_tokens: Option<u64>,
}

/// 工具调用信息 — 工具ID、名称和输入参数
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsToolCallInfo {
    pub id: String,
    pub name: String,
    #[ts(type = "Record<string, unknown>")]
    pub input: serde_json::Value,
}

/// 工具执行信息 — 轮次、工具名、输入、结果和执行耗时
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsToolExecutionInfo {
    pub round: usize,
    pub tool_name: String,
    #[ts(type = "Record<string, unknown>")]
    pub tool_input: serde_json::Value,
    pub tool_result: String,
    pub duration_ms: u128,
}

/// 会话信息 — ID、标题、关联Agent和时间戳
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsConversation {
    pub id: String,
    pub title: String,
    pub agent_id: Option<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

/// 消息信息 — 角色、内容、工具调用和元数据
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsMessage {
    pub id: String,
    pub conversation_id: String,
    pub role: String,
    pub content: String,
    pub timestamp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_name: Option<String>,
    #[ts(type = "Record<string, unknown> | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_input: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[ts(type = "Record<string, unknown> | undefined")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<serde_json::Value>,
    #[serde(default)]
    pub is_error: bool,
}

/// Agent信息 — ID、显示名称、描述、用途和范围
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[ts(export)]
pub struct TsAgentInfo {
    pub id: String,
    pub display_name: String,
    pub description: Option<String>,
    pub purpose: Option<String>,
    pub scope: Option<String>,
}
