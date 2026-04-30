// Claw Desktop - 路径解析器 - 解析配置文件和数据目录路径
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

#[cfg(feature = "tauri-integration")]
use tauri::Manager;

static APP_ROOT: OnceLock<PathBuf> = OnceLock::new();
static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();

/// 运行模式枚举 - 区分开发模式和生产模式
#[derive(Debug, Clone)]
pub enum RunMode {
    Dev,        // 开发模式（使用.temp_build目录）
    Production, // 生产模式（使用exe所在目录或app_data_dir）
}

/// 初始化路径解析器（Tauri集成模式）
/// 解析应用根目录并创建必要的目录结构
#[cfg(feature = "tauri-integration")]
pub fn init(app: &tauri::App) -> Result<PathBuf, String> {
    let app_handle = app.handle().clone();
    let _ = APP_HANDLE.set(app_handle);

    let root = resolve_app_root(app.handle())?;
    let _ = APP_ROOT.set(root.clone());
    std::fs::create_dir_all(&root).map_err(|e| format!("Failed to create app root dir: {}", e))?;
    log::info!(
        "[PathResolver] Initialized | mode={:?} | root={}",
        get_run_mode(),
        root.display()
    );
    Ok(root)
}

/// 获取全局AppHandle引用
#[cfg(feature = "tauri-integration")]
pub fn app_handle() -> &'static tauri::AppHandle {
    APP_HANDLE
        .get()
        .expect("PathResolver not initialized. Call init() first.")
}

/// 检查路径解析器是否已初始化
pub fn is_initialized() -> bool {
    APP_ROOT.get().is_some()
}

/// 初始化路径解析器（非Tauri模式，用于测试或CLI）
#[cfg(not(feature = "tauri-integration"))]
pub fn init(_app_root: &Path) -> Result<PathBuf, String> {
    let root = _app_root.to_path_buf();
    let _ = APP_ROOT.set(root.clone());
    std::fs::create_dir_all(&root).map_err(|e| format!("Failed to create app root dir: {}", e))?;
    log::info!(
        "[PathResolver] Initialized (non-tauri) | root={}",
        root.display()
    );
    Ok(root)
}

/// 解析应用根目录 - 开发模式使用.temp_build，生产模式使用exe目录或app_data_dir
#[cfg(feature = "tauri-integration")]
fn resolve_app_root(app_handle: &tauri::AppHandle) -> Result<PathBuf, String> {
    let run_mode = detect_run_mode();

    match run_mode {
        RunMode::Dev => {
            let src_tauri_dir = find_src_tauri_dir();
            let dev_root = if let Some(ref dir) = src_tauri_dir {
                dir.join(".temp_build")
            } else if let Ok(cwd) = std::env::current_dir() {
                cwd.join(".temp_build")
            } else {
                return Err("Cannot determine source directory for dev mode".to_string());
            };
            log::info!(
                "[PathResolver] Dev mode, using .temp_build root: {}",
                dev_root.display()
            );
            Ok(dev_root)
        }
        RunMode::Production => {
            if let Ok(exe) = std::env::current_exe() {
                if let Some(parent) = exe.parent() {
                    log::info!(
                        "[PathResolver] Production mode, using exe dir: {}",
                        parent.display()
                    );
                    return Ok(parent.to_path_buf());
                }
            }
            if let Ok(app_data_dir) = app_handle.path().app_data_dir() {
                log::info!(
                    "[PathResolver] Production mode fallback, using app_data_dir: {}",
                    app_data_dir.display()
                );
                return Ok(app_data_dir);
            }
            Err("Cannot determine installation directory for production mode".to_string())
        }
    }
}

/// 查找src-tauri目录 - 从当前工作目录或可执行文件路径向上搜索
fn find_src_tauri_dir() -> Option<PathBuf> {
    if let Ok(cwd) = std::env::current_dir() {
        if cwd.file_name().map(|n| n == "src-tauri").unwrap_or(false) {
            return Some(cwd.clone());
        }
        if cwd.join("src-tauri").is_dir() {
            return Some(cwd.join("src-tauri"));
        }
        if let Some(parent) = cwd.parent() {
            if parent.join("src-tauri").is_dir() {
                return Some(parent.join("src-tauri"));
            }
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(exe_dir) = exe.parent() {
            if exe_dir
                .file_name()
                .map(|n| n == "src-tauri")
                .unwrap_or(false)
            {
                return Some(exe_dir.to_path_buf());
            }
            if exe_dir.join("src-tauri").is_dir() {
                return Some(exe_dir.join("src-tauri"));
            }
            if let Some(parent) = exe_dir.parent() {
                if parent.join("src-tauri").is_dir() {
                    return Some(parent.join("src-tauri"));
                }
            }
        }
    }
    None
}

/// 检测当前运行模式 - 根据debug_assertions标志和目录结构判断
fn detect_run_mode() -> RunMode {
    if cfg!(debug_assertions) {
        return RunMode::Dev;
    }

    if let Ok(cwd) = std::env::current_dir() {
        if cwd.join(".temp_build").exists() || cwd.join("src-tauri").exists() {
            return RunMode::Dev;
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            if parent.join(".temp_build").exists() || parent.join("src-tauri").exists() {
                return RunMode::Dev;
            }
        }
    }

    RunMode::Production
}

/// 获取当前运行模式
pub fn get_run_mode() -> RunMode {
    detect_run_mode()
}

/// 获取应用根目录路径
pub fn get_app_root() -> &'static PathBuf {
    APP_ROOT
        .get()
        .expect("PathResolver not initialized. Call init() first.")
}

/// 获取数据库目录路径
pub fn db_dir() -> PathBuf {
    get_app_root().join("db")
}

/// 获取主数据库文件路径
pub fn db_path() -> PathBuf {
    db_dir().join("claw.db")
}

/// 获取Agent隔离数据库文件路径
pub fn agent_db_path() -> PathBuf {
    db_dir().join("agent_isolation.db")
}

/// 获取技能目录路径
pub fn skills_dir() -> PathBuf {
    get_app_root().join("skills")
}

/// 获取技能搜索路径列表 - 包含内置技能目录和用户自定义技能目录
#[allow(dead_code)]
pub fn skills_search_paths() -> Vec<(PathBuf, &'static str)> {
    let mut paths = Vec::new();

    paths.push((skills_dir(), "Bundled"));

    if let Some(home) = dirs::home_dir() {
        paths.push((home.join(".claw-desktop").join("skills"), "User"));
    }

    paths
}

/// 获取配置文件路径
pub fn config_path() -> PathBuf {
    get_app_root().join("config.toml")
}

/// 获取日志目录路径
pub fn logs_dir() -> PathBuf {
    get_app_root().join("logs")
}

/// 获取扩展目录路径
pub fn extensions_dir() -> PathBuf {
    get_app_root().join("extensions")
}

/// 获取JWT密钥文件路径
pub fn jwt_secret_path() -> PathBuf {
    get_app_root().join("jwt_secret.txt")
}

/// 获取RSA密钥对文件路径
pub fn rsa_keypair_path() -> PathBuf {
    get_app_root().join("rsa_keypair.pem")
}

const DEFAULT_CONFIG_TOML: &str = r#"[app]
language = "zh-CN"
theme = "dark"
auto_update = true
minimize_to_tray = false
startup_behavior = "normal"

[model]
default_model = "claude-sonnet-4-20250514"
provider = "anthropic"
custom_url = ""
custom_api_key = ""
custom_model_name = ""
temperature = 0.7
max_tokens = 16384
top_p = 1.0
thinking_budget = 10000
stream_mode = true

[api]
api_key = ""
base_url = "https://api.anthropic.com"
api_version = "2023-06-01"
timeout_seconds = 120
retry_count = 3

[ui]
font_size = 14
font_family = "Inter"
sidebar_width = 280
show_line_numbers = true
code_theme = "oneDark"
message_style = "bubble"

[advanced]
data_dir = ""
log_level = "info"
max_conversation_history = 100
auto_compact_tokens = 150000
proxy_url = ""
enable_telemetry = false

[database]
backend = "sqlite"
initialized = false

[database.sqlite]
enable_vec = true

[database.postgres]
host = "localhost"
port = 5432
database = "claw_desktop"

[database.qdrant]
url = "http://localhost:6333"
collection = "claw_vectors"
"#;

/// 确保默认配置文件存在，不存在则创建
fn ensure_default_config(config_path: &Path) -> Result<bool, String> {
    if config_path.exists() {
        return Ok(false);
    }

    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("Failed to create config dir: {}", e))?;
    }

    std::fs::write(config_path, DEFAULT_CONFIG_TOML)
        .map_err(|e| format!("Failed to write default config: {}", e))?;

    log::info!(
        "[PathResolver] Created default config.toml @ {}",
        config_path.display()
    );
    Ok(true)
}

type ExportSkillsFn = Option<fn(target_dir: &Path) -> Result<usize, String>>;

static EXPORT_SKILLS_CALLBACK: OnceLock<ExportSkillsFn> = OnceLock::new();

/// 注册技能导出回调函数
pub fn register_skills_export_callback(callback: fn(target_dir: &Path) -> Result<usize, String>) {
    let _ = EXPORT_SKILLS_CALLBACK.set(Some(callback));
}

/// 确保所有必要目录存在，创建默认配置文件，导出内置技能
pub fn ensure_dirs() -> Result<(), String> {
    let root = get_app_root();
    let dirs = [
        root.as_path(),
        &db_dir(),
        &skills_dir(),
        &logs_dir(),
        &extensions_dir(),
    ];

    for dir in &dirs {
        std::fs::create_dir_all(dir)
            .map_err(|e| format!("Failed to create dir {}: {}", dir.display(), e))?;
    }

    let cfg_path = config_path();
    let config_created = ensure_default_config(&cfg_path)?;

    let skills_target = skills_dir();
    let has_any_skill = skills_target
        .read_dir()
        .ok()
        .and_then(|mut entries| entries.next().map(|_| true))
        .unwrap_or(false);

    let exported_skills = if !has_any_skill {
        if let Some(callback) = EXPORT_SKILLS_CALLBACK.get().and_then(|f| *f) {
            match callback(&skills_target) {
                Ok(count) => count,
                Err(e) => {
                    log::warn!("[PathResolver] Failed to export bundled skills: {}", e);
                    0
                }
            }
        } else {
            0
        }
    } else {
        0
    };

    log::info!(
        "[PathResolver] All directories ensured | root={} | config_created={} | skills_exported={}",
        root.display(),
        config_created,
        exported_skills
    );
    Ok(())
}
