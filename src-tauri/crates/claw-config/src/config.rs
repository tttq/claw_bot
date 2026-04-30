// Claw Desktop - 配置核心 - AppConfig结构体和配置文件解析
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use std::sync::OnceLock;
use tokio::sync::OnceCell as AsyncOnceCell;

use crate::path_resolver;

static APP_CONFIG: OnceLock<AppConfig> = OnceLock::new();
static INITIALIZED: AsyncOnceCell<Result<(), String>> = AsyncOnceCell::const_new();

/// 应用配置根结构体 - 包含所有配置子项
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub app: AppSettings,           // 应用基础设置
    pub model: ModelSettings,       // 模型设置
    pub api: ApiSettings,           // API设置
    pub ui: UiSettings,             // UI设置
    pub advanced: AdvancedSettings, // 高级设置
    #[serde(default)]
    pub harness: HarnessSettings, // Agent管理设置
    #[serde(default)]
    pub tools: ToolSettings, // 工具权限设置
    #[serde(default)]
    pub database: DatabaseSettings, // 数据库设置
}

/// 应用基础设置 - 语言、主题、启动行为等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppSettings {
    #[serde(default = "default_language")]
    pub language: String, // 界面语言
    #[serde(default = "default_theme")]
    pub theme: String, // 主题 (dark/light/system)
    #[serde(default)]
    pub auto_update: bool, // 自动更新
    #[serde(default)]
    pub minimize_to_tray: bool, // 最小化到托盘
    #[serde(default = "default_startup")]
    pub startup_behavior: String, // 启动行为 (normal/minimized)
}

/// 模型设置 - 默认模型、服务商、参数等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelSettings {
    #[serde(default = "default_model")]
    pub default_model: String, // 默认模型名称
    #[serde(default = "default_provider")]
    pub provider: String, // 服务商 (anthropic/openai/custom)
    #[serde(default)]
    pub custom_url: String, // 自定义API URL
    #[serde(default)]
    pub custom_api_key: String, // 自定义API Key
    #[serde(default)]
    pub custom_model_name: String, // 自定义模型名称
    #[serde(default = "default_temperature")]
    pub temperature: f64, // 温度参数 (0.0-2.0)
    #[serde(default = "default_max_tokens")]
    pub max_tokens: u32, // 最大生成token数
    #[serde(default = "default_top_p")]
    pub top_p: f64, // Top-P采样参数
    #[serde(default)]
    pub thinking_budget: u64, // 思维预算token数
    #[serde(default = "default_true")]
    pub stream_mode: bool, // 是否启用流式模式
    #[serde(default = "default_api_format")]
    pub api_format: String, // API格式 (anthropic/openai)
}

/// API设置 - 密钥、基础URL、超时等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiSettings {
    #[serde(default)]
    pub api_key: String, // API密钥
    #[serde(default = "default_base_url")]
    pub base_url: String, // API基础URL
    #[serde(default = "default_api_version")]
    pub api_version: String, // API版本
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u64, // 请求超时时间（秒）
    #[serde(default = "default_retry")]
    pub retry_count: u32, // 重试次数
}

/// UI设置 - 字体、侧边栏、代码主题等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    #[serde(default = "default_font_size")]
    pub font_size: u32, // 字体大小
    #[serde(default = "default_font_family")]
    pub font_family: String, // 字体族
    #[serde(default = "default_sidebar_width")]
    pub sidebar_width: u32, // 侧边栏宽度
    #[serde(default)]
    pub show_line_numbers: bool, // 显示行号
    #[serde(default = "default_code_theme")]
    pub code_theme: String, // 代码高亮主题
    #[serde(default = "default_msg_style")]
    pub message_style: String, // 消息样式 (bubble/flat)
    #[serde(default = "default_true")]
    pub show_tool_executions: bool, // 显示工具执行详情
}

/// 高级设置 - 数据目录、日志级别、代理等
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AdvancedSettings {
    #[serde(default)]
    pub data_dir: String, // 数据存储目录
    #[serde(default = "default_log_level")]
    pub log_level: String, // 日志级别
    #[serde(default = "default_max_history")]
    pub max_conversation_history: u32, // 最大对话历史数
    #[serde(default = "default_compact_threshold")]
    pub auto_compact_tokens: u64, // 自动压缩token阈值
    #[serde(default)]
    pub proxy_url: String, // 代理URL
    #[serde(default = "default_true")]
    pub enable_telemetry: bool, // 启用遥测
    #[serde(default)]
    pub plan_mode: bool, // 计划模式开关
}

/// Agent管理设置 - 错误学习、跨记忆、验证等
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HarnessSettings {
    #[serde(default)]
    pub error_learning_enabled: bool, // 启用错误学习
    #[serde(default = "default_true")]
    pub cross_memory_enabled: bool, // 启用跨Agent记忆共享
    #[serde(default = "default_true")]
    pub validation_enabled: bool, // 启用验证
    #[serde(default = "default_visibility")]
    pub default_memory_visibility: String, // 默认记忆可见性
    #[serde(default = "default_parallel")]
    pub max_parallel_subtasks: usize, // 最大并行子任务数
    #[serde(default = "default_task_timeout")]
    pub task_timeout_seconds: u64, // 任务超时时间（秒）
    #[serde(default = "default_true")]
    pub agents_md_auto_refresh: bool, // 自动刷新AGENTS.md
}

/// 工具权限设置 - 控制各类工具的启用/禁用
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolSettings {
    #[serde(default = "default_true")]
    pub file_access: bool, // 文件读取权限
    #[serde(default = "default_true")]
    pub file_write: bool, // 文件写入权限
    #[serde(default = "default_true")]
    pub shell: bool, // Shell命令权限
    #[serde(default = "default_true")]
    pub search: bool, // 搜索权限
    #[serde(default = "default_true")]
    pub web: bool, // 网络访问权限
    #[serde(default = "default_true")]
    pub git: bool, // Git操作权限
    #[serde(default = "default_true")]
    pub browser: bool, // 浏览器权限
    #[serde(default = "default_true")]
    pub automation: bool, // 自动化权限
    #[serde(default = "default_true")]
    pub agent: bool, // Agent权限
}

/// 工具权限默认值 — 所有工具默认启用
impl Default for ToolSettings {
    fn default() -> Self {
        Self {
            file_access: true,
            file_write: true,
            shell: true,
            search: true,
            web: true,
            git: true,
            browser: true,
            automation: true,
            agent: true,
        }
    }
}

/// 数据库设置 - 支持SQLite/PostgreSQL/Qdrant后端
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DatabaseSettings {
    #[serde(default = "default_db_backend")]
    pub backend: String, // 数据库后端类型 (sqlite/postgres/qdrant)
    #[serde(default)]
    pub sqlite: SqliteSettings, // SQLite配置
    #[serde(default)]
    pub postgres: PostgresSettings, // PostgreSQL配置
    #[serde(default)]
    pub qdrant: QdrantSettings, // Qdrant向量数据库配置
    #[serde(default)]
    pub initialized: bool, // 数据库是否已初始化
}

/// SQLite设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SqliteSettings {
    #[serde(default = "default_true")]
    pub enable_vec: bool, // 启用向量扩展
    #[serde(default)]
    pub db_path: String, // 数据库文件路径
}

/// PostgreSQL设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PostgresSettings {
    #[serde(default = "default_pg_host")]
    pub host: String, // 主机地址
    #[serde(default = "default_pg_port")]
    pub port: u16, // 端口号
    #[serde(default = "default_pg_database")]
    pub database: String, // 数据库名称
    #[serde(default)]
    pub username: String, // 用户名
    #[serde(default)]
    pub password: String, // 密码
    #[serde(default)]
    pub pool_size: u32, // 连接池大小
}

/// Qdrant向量数据库设置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QdrantSettings {
    #[serde(default = "default_qdrant_url")]
    pub url: String, // Qdrant服务URL
    #[serde(default)]
    pub api_key: String, // API密钥
    #[serde(default = "default_qdrant_collection")]
    pub collection: String, // 集合名称
}

/// 数据库默认后端 — SQLite
fn default_db_backend() -> String {
    "sqlite".to_string()
}
/// PostgreSQL默认主机 — localhost
fn default_pg_host() -> String {
    "localhost".to_string()
}
/// PostgreSQL默认端口 — 5432
fn default_pg_port() -> u16 {
    5432
}
/// PostgreSQL默认数据库名 — claw_desktop
fn default_pg_database() -> String {
    "claw_desktop".to_string()
}
/// Qdrant默认URL — http://localhost:6333
fn default_qdrant_url() -> String {
    "http://localhost:6333".to_string()
}
/// Qdrant默认集合名 — claw_vectors
fn default_qdrant_collection() -> String {
    "claw_vectors".to_string()
}

/// 数据库设置默认值 — SQLite后端
impl Default for DatabaseSettings {
    fn default() -> Self {
        Self {
            backend: default_db_backend(),
            sqlite: SqliteSettings::default(),
            postgres: PostgresSettings::default(),
            qdrant: QdrantSettings::default(),
            initialized: false,
        }
    }
}

/// SQLite设置默认值 — 启用向量扩展
impl Default for SqliteSettings {
    fn default() -> Self {
        Self {
            enable_vec: true,
            db_path: String::new(),
        }
    }
}

/// PostgreSQL设置默认值 — localhost:5432/claw_desktop
impl Default for PostgresSettings {
    fn default() -> Self {
        Self {
            host: default_pg_host(),
            port: default_pg_port(),
            database: default_pg_database(),
            username: String::new(),
            password: String::new(),
            pool_size: 5,
        }
    }
}

/// Qdrant设置默认值 — localhost:6333/claw_vectors
impl Default for QdrantSettings {
    fn default() -> Self {
        Self {
            url: default_qdrant_url(),
            api_key: String::new(),
            collection: default_qdrant_collection(),
        }
    }
}

impl DatabaseSettings {
    /// 判断是否使用SQLite后端
    pub fn is_sqlite(&self) -> bool {
        self.backend == "sqlite"
    }

    /// 判断是否使用PostgreSQL后端
    pub fn is_postgres(&self) -> bool {
        self.backend == "postgres"
    }

    /// 判断是否使用Qdrant向量数据库后端
    pub fn is_qdrant(&self) -> bool {
        self.backend == "qdrant"
    }

    /// 获取数据库连接URL（仅PostgreSQL后端有效）
    pub fn connection_url(&self) -> String {
        match self.backend.as_str() {
            "postgres" => {
                format!(
                    "postgres://{}:{}@{}:{}/{}",
                    self.postgres.username,
                    self.postgres.password,
                    self.postgres.host,
                    self.postgres.port,
                    self.postgres.database
                )
            }
            _ => String::new(),
        }
    }
}

/// 默认可见性 — public
fn default_visibility() -> String {
    "public".to_string()
}
/// 默认并行子任务数 — 5
fn default_parallel() -> usize {
    5
}
/// 默认任务超时 — 300秒
fn default_task_timeout() -> u64 {
    300
}

/// 默认语言 — 简体中文
fn default_language() -> String {
    "zh-CN".to_string()
}
/// 默认主题 — 暗色
fn default_theme() -> String {
    "dark".to_string()
}
/// 默认启动行为 — 正常启动
fn default_startup() -> String {
    "normal".to_string()
}
/// 默认模型 — claude-sonnet-4
fn default_model() -> String {
    "claude-sonnet-4-20250514".to_string()
}
/// 默认提供商 — anthropic
fn default_provider() -> String {
    "anthropic".to_string()
}
/// 默认温度 — 0.7
fn default_temperature() -> f64 {
    0.7
}
/// 默认最大Token — 16384
fn default_max_tokens() -> u32 {
    16384
}
/// 默认Top-P — 1.0
fn default_top_p() -> f64 {
    1.0
}
/// 默认布尔值 — true
fn default_true() -> bool {
    true
}
/// 默认API格式 — anthropic
fn default_api_format() -> String {
    "anthropic".to_string()
}
/// 默认API基础URL — Anthropic官方
fn default_base_url() -> String {
    "https://api.anthropic.com".to_string()
}
/// 默认API版本 — 2023-06-01
fn default_api_version() -> String {
    "2023-06-01".to_string()
}
/// 默认超时 — 120秒
fn default_timeout() -> u64 {
    120
}
/// 默认重试次数 — 3
fn default_retry() -> u32 {
    3
}
/// 默认字体大小 — 14px
fn default_font_size() -> u32 {
    14
}
/// 默认字体 — Inter
fn default_font_family() -> String {
    "Inter".to_string()
}
/// 默认侧边栏宽度 — 280px
fn default_sidebar_width() -> u32 {
    280
}
/// 默认代码主题 — oneDark
fn default_code_theme() -> String {
    "oneDark".to_string()
}
/// 默认消息样式 — 气泡
fn default_msg_style() -> String {
    "bubble".to_string()
}
/// 默认日志级别 — info
fn default_log_level() -> String {
    "info".to_string()
}
/// 默认最大历史记录 — 100条
fn default_max_history() -> u32 {
    100
}
/// 默认压缩阈值 — 150000 Token
fn default_compact_threshold() -> u64 {
    150000
}

/// 应用配置默认值 — 简体中文/暗色主题/Claude Sonnet/Anthropic
impl Default for AppConfig {
    fn default() -> Self {
        Self {
            app: AppSettings {
                language: default_language(),
                theme: default_theme(),
                auto_update: true,
                minimize_to_tray: false,
                startup_behavior: default_startup(),
            },
            model: ModelSettings {
                default_model: default_model(),
                provider: default_provider(),
                custom_url: String::new(),
                custom_api_key: String::new(),
                custom_model_name: String::new(),
                temperature: default_temperature(),
                max_tokens: default_max_tokens(),
                top_p: default_top_p(),
                thinking_budget: 10000,
                stream_mode: true,
                api_format: default_api_format(),
            },
            api: ApiSettings {
                api_key: String::new(),
                base_url: default_base_url(),
                api_version: default_api_version(),
                timeout_seconds: default_timeout(),
                retry_count: default_retry(),
            },
            ui: UiSettings {
                font_size: default_font_size(),
                font_family: default_font_family(),
                sidebar_width: default_sidebar_width(),
                show_line_numbers: true,
                code_theme: default_code_theme(),
                message_style: default_msg_style(),
                show_tool_executions: true,
            },
            advanced: AdvancedSettings {
                data_dir: String::new(),
                log_level: default_log_level(),
                max_conversation_history: default_max_history(),
                auto_compact_tokens: default_compact_threshold(),
                proxy_url: String::new(),
                enable_telemetry: false,
                plan_mode: false,
            },
            harness: HarnessSettings {
                error_learning_enabled: false,
                cross_memory_enabled: true,
                validation_enabled: true,
                default_memory_visibility: default_visibility(),
                max_parallel_subtasks: default_parallel(),
                task_timeout_seconds: default_task_timeout(),
                agents_md_auto_refresh: true,
            },
            tools: ToolSettings::default(),
            database: DatabaseSettings::default(),
        }
    }
}

impl AppConfig {
    /// 从配置文件加载配置，不存在则创建默认配置
    pub fn load_or_create(data_dir: &Path) -> Result<Self> {
        let config_path = data_dir.join("config.toml");

        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            let config: AppConfig = toml::from_str(&content)?;

            let mut config = config;
            config.advanced.data_dir = data_dir.to_string_lossy().to_string();

            config.validate()?;
            Ok(config)
        } else {
            let config = AppConfig::default();
            config.save(data_dir)?;
            Ok(config)
        }
    }

    /// 保存配置到指定数据目录
    pub fn save(&self, data_dir: &Path) -> Result<()> {
        self.save_to_path(&data_dir.join("config.toml"))
    }

    /// 保存配置到指定路径
    pub fn save_to_path(&self, config_path: &Path) -> Result<()> {
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;

        if config_path.exists() {
            let metadata = fs::metadata(config_path)?;
            let mut permissions = metadata.permissions();
            permissions.set_readonly(false);
            fs::set_permissions(config_path, permissions)?;
        }

        fs::write(config_path, content)?;
        log::info!("[Config] Saved config to {}", config_path.display());
        Ok(())
    }

    /// 验证配置合法性 - 检查温度、token数、主题等参数范围
    pub fn validate(&self) -> Result<()> {
        if self.model.temperature < 0.0 || self.model.temperature > 2.0 {
            anyhow::bail!("Temperature must be between 0 and 2");
        }
        if self.model.max_tokens < 1 || self.model.max_tokens > 200000 {
            anyhow::bail!("Max tokens must be between 1 and 200000");
        }
        if self.model.top_p < 0.0 || self.model.top_p > 1.0 {
            anyhow::bail!("Top P must be between 0 and 1");
        }
        if self.api.timeout_seconds == 0 {
            anyhow::bail!("Timeout must be greater than 0");
        }
        if self.ui.font_size < 8 || self.ui.font_size > 32 {
            anyhow::bail!("Font size must be between 8 and 32");
        }
        if !["dark", "light", "system"].contains(&self.app.theme.as_str()) {
            anyhow::bail!("Theme must be one of: dark, light, system");
        }
        if !["anthropic", "openai", "custom"].contains(&self.model.provider.as_str()) {
            anyhow::bail!("Provider must be one of: anthropic, openai, custom");
        }
        if !["openai", "anthropic"].contains(&self.model.api_format.as_str()) {
            anyhow::bail!("API format must be one of: openai, anthropic");
        }
        if !["sqlite", "postgres", "qdrant"].contains(&self.database.backend.as_str()) {
            anyhow::bail!("Database backend must be one of: sqlite, postgres, qdrant");
        }
        if self.database.is_postgres() && self.database.postgres.username.is_empty() {
            anyhow::bail!("PostgreSQL username is required when using postgres backend");
        }
        Ok(())
    }

    /// 获取API Key - 优先使用自定义Key，其次使用全局Key
    pub fn get_api_key(&self) -> Option<&str> {
        if !self.model.custom_api_key.is_empty() {
            Some(&self.model.custom_api_key)
        } else if !self.api.api_key.is_empty() {
            Some(&self.api.api_key)
        } else {
            None
        }
    }

    /// 解析API Key - 依次从配置、环境变量中获取
    pub fn resolve_api_key(&self) -> Result<String> {
        if let Some(key) = self.get_api_key() {
            return Ok(key.to_string());
        }

        if let Ok(key) = std::env::var("ANTHROPIC_API_KEY") {
            return Ok(key);
        }
        if let Ok(key) = std::env::var("OPENAI_API_KEY") {
            return Ok(key);
        }
        if let Ok(key) = std::env::var("LLM_API_KEY") {
            return Ok(key);
        }

        anyhow::bail!("No API key configured. Please set it in Settings or environment variables.")
    }

    /// 获取API基础URL - 优先使用自定义URL，其次使用配置URL，最后使用默认URL
    pub fn get_base_url(&self) -> &str {
        if !self.model.custom_url.is_empty() {
            &self.model.custom_url
        } else if !self.api.base_url.is_empty() {
            &self.api.base_url
        } else {
            match self.model.provider.as_str() {
                "openai" => "https://api.openai.com/v1",
                _ => "https://api.anthropic.com",
            }
        }
    }

    /// 获取API格式（anthropic/openai）
    pub fn get_api_format(&self) -> &str {
        &self.model.api_format
    }

    /// 判断是否使用OpenAI兼容的API格式
    pub fn is_openai_compatible(&self) -> bool {
        self.get_api_format() == "openai"
    }
}

/// 确保配置已初始化（懒加载，首次调用时自动初始化）
async fn ensure_initialized() -> Result<(), String> {
    INITIALIZED
        .get_or_init(|| async {
            if !path_resolver::is_initialized() {
                return Err(
                    "PathResolver not initialized. Call claw_config::init(app) first.".to_string(),
                );
            }

            path_resolver::ensure_dirs().map_err(|e| format!("Ensure dirs failed: {}", e))?;

            let config =
                AppConfig::load_or_create(path_resolver::get_app_root()).unwrap_or_default();

            let _ = APP_CONFIG.set(config);

            log::info!("[Config] Auto-initialized successfully");
            Ok(())
        })
        .await
        .clone()
}

/// 获取全局配置引用（异步，自动初始化）
pub async fn get_config() -> Result<&'static AppConfig, String> {
    ensure_initialized().await?;
    Ok(APP_CONFIG
        .get()
        .expect("Config should be initialized after ensure_initialized"))
}

/// 尝试获取全局配置引用（不触发初始化）
pub fn try_get_config() -> Option<&'static AppConfig> {
    APP_CONFIG.get()
}

/// 检查配置是否已初始化
pub fn is_initialized() -> bool {
    APP_CONFIG.get().is_some()
}
