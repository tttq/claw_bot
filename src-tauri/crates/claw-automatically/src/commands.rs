// Claw Desktop - 桌面自动化 Tauri 命令
// 注册所有自动化相关的Tauri命令：引擎初始化、指令执行、屏幕截图/OCR、
// 鼠标键盘操作、窗口管理、应用启动、Mano-P模型管理等
use std::sync::{Mutex, Arc};
use once_cell::sync::Lazy;
use async_trait::async_trait;
use crate::{AutomaticallyEngine, AutomaticallyConfig};

static ENGINE: Lazy<Mutex<Option<Arc<AutomaticallyEngine>>>> = Lazy::new(|| Mutex::new(None));

/// 初始化自动化引擎 — 使用指定配置创建引擎并注册到全局执行器
pub fn init_engine_with_config(config: AutomaticallyConfig) -> Result<(), String> {
    let engine = AutomaticallyEngine::new(config);

    let mut engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    *engine_guard = Some(Arc::new(engine));

    let engine_ref = engine_guard.as_ref()
        .ok_or_else(|| "Failed to get engine reference after initialization".to_string())?
        .clone();
    claw_traits::automation::set_executor(ArcWrapper(engine_ref))
        .map_err(|e| format!("Failed to register automation executor: {}", e))?;

    Ok(())
}

/// Arc包装器 — 将AutomaticallyEngine包装为AutomationExecutor trait实现
struct ArcWrapper(Arc<AutomaticallyEngine>);

/// 获取当前系统进程名列表 — 用于启动应用前后对比验证
#[cfg(target_os = "windows")]
fn get_process_names() -> Vec<String> {
    use std::process::Command;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command",
            "Get-Process | Where-Object { $_.MainWindowTitle -ne '' -or $_.Name -match '\\.(exe)$' -or $_.Path } | Select-Object -ExpandProperty ProcessName"])
        .output();
    match output {
        Ok(out) => {
            String::from_utf8_lossy(&out.stdout)
                .lines()
                .map(|l| l.trim().to_string())
                .filter(|l| !l.is_empty())
                .collect()
        }
        Err(_) => Vec::new(),
    }
}

#[cfg(not(target_os = "windows"))]
fn get_process_names() -> Vec<String> {
    Vec::new()
}

#[async_trait]
impl claw_traits::automation::AutomationExecutor for ArcWrapper {
    /// 执行自动化指令 — 通过引擎解析并执行自然语言指令
    async fn execute_automation(&self, instruction: &str) -> Result<String, String> {
        self.0.execute_instruction(instruction)
            .await
            .map(|r| serde_json::to_string(&r).unwrap_or_else(|_| r.success.to_string()))
            .map_err(|e| e.to_string())
    }
    /// 截屏OCR识别 — 捕获屏幕并识别文字内容，返回结构化JSON
    async fn capture_screen(&self) -> Result<String, String> {
        let frame = crate::capture::screen::capture_screen().map_err(|e| e.to_string())?;
        let screen_size = serde_json::json!([frame.width, frame.height]);
        let image_base64 = frame.to_base64();

        let ocr_text = match crate::capture::screen::ocr_screen_text() {
            Ok(text) => text,
            Err(e) => {
                log::warn!("[ArcWrapper:capture_screen] OCR failed: {}, continuing without OCR", e);
                "[OCR not available]".to_string()
            }
        };

        let result = serde_json::json!({
            "ocr_summary": ocr_text,
            "screen_size": screen_size,
            "image_base64": image_base64,
        });

        Ok(serde_json::to_string(&result).unwrap_or_else(|_| ocr_text))
    }
    /// 鼠标左键单击 — 在指定坐标点击
    async fn mouse_click(&self, x: f64, y: f64) -> Result<String, String> {
        self.0.mouse_click(x, y).await.map_err(|e| e.to_string())?;
        Ok(format!("Clicked at ({}, {})", x, y))
    }
    /// 鼠标左键双击 — 在指定坐标双击
    async fn mouse_double_click(&self, x: f64, y: f64) -> Result<String, String> {
        self.0.mouse_double_click(x, y).await.map_err(|e| e.to_string())?;
        Ok(format!("Double-clicked at ({}, {})", x, y))
    }
    /// 鼠标右键点击 — 在指定坐标右键点击
    async fn mouse_right_click(&self, x: f64, y: f64) -> Result<String, String> {
        self.0.mouse_right_click(x, y).await.map_err(|e| e.to_string())?;
        Ok(format!("Right-clicked at ({}, {})", x, y))
    }
    /// 键盘输入文本 — 模拟键盘逐字输入
    async fn keyboard_type(&self, text: &str) -> Result<String, String> {
        self.0.keyboard_type(text).await.map_err(|e| e.to_string())?;
        Ok(format!("Typed: {} chars", text.len()))
    }
    /// 键盘按键 — 模拟按下指定按键
    async fn keyboard_press(&self, key: &str) -> Result<String, String> {
        self.0.keyboard_press(key).await.map_err(|e| e.to_string())?;
        Ok(format!("Pressed: {}", key))
    }
    /// 列出已安装应用 — 可选按关键词过滤
    async fn list_installed_apps(&self, filter: Option<&str>) -> Result<String, String> {
        let apps = self.0.list_installed_apps(filter).map_err(|e| e.to_string())?;
        serde_json::to_string(&apps).map_err(|e| e.to_string())
    }
    /// 启动应用 — 按名称查找并启动，启动后验证进程是否存在
    async fn launch_application(&self, name: &str) -> Result<String, String> {
        let processes_before = get_process_names();

        self.0.launch_application(name).map_err(|e| e.to_string())?;

        tokio::time::sleep(tokio::time::Duration::from_millis(2000)).await;

        let processes_after = get_process_names();

        let lower_name = name.to_lowercase();
        let new_processes: Vec<String> = processes_after
            .into_iter()
            .filter(|p| !processes_before.contains(p))
            .filter(|p| p.to_lowercase().contains(&lower_name) || lower_name.contains(&p.to_lowercase().replace(".exe", "")))
            .collect();

        if !new_processes.is_empty() {
            log::info!("[ArcWrapper:launch_application] New process detected: {:?}", new_processes);
            Ok(format!("Launched: {} (active window: {})", name, new_processes.join(", ")))
        } else {
            log::info!("[ArcWrapper:launch_application] Launch command sent for '{}', but no new process detected", name);
            Ok(format!("Launch command sent for: {}. The application may need more time to start, or it may not be installed correctly.", name))
        }
    }
    /// OCR识别屏幕 — 捕获屏幕并识别文字
    async fn ocr_recognize_screen(&self, _language: Option<&str>) -> Result<String, String> {
        let ocr_text = crate::capture::screen::ocr_screen_text().map_err(|e| e.to_string())?;
        Ok(ocr_text)
    }
}

/// 初始化自动化引擎 — 使用默认配置
#[tauri::command]
pub fn init_automatically_engine() -> Result<(), String> {
    init_engine_with_config(AutomaticallyConfig::default())
}

/// 初始化自动化引擎 — 使用自定义配置
#[tauri::command]
pub fn init_automatically_engine_with_config(config: AutomaticallyConfig) -> Result<(), String> {
    init_engine_with_config(config)
}

/// 执行自动化指令 — 解析自然语言指令并返回执行结果
#[tauri::command]
pub async fn execute_automation_instruction(instruction: String) -> Result<serde_json::Value, String> {
    let engine_arc = {
        let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        engine_guard.clone()
    };

    match engine_arc {
        Some(engine) => {
            let result = engine.execute_instruction(&instruction).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "success": result.success,
                "instruction": result.instruction,
                "intent": result.intent,
                "confidence": result.confidence,
                "inference_time_ms": result.inference_time_ms,
                "action_count": result.action_count,
                "error": result.error_message,
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}

/// 配置Mano-P云端API — 设置API地址和密钥
#[tauri::command]
pub async fn configure_mano_p_cloud(api_url: String, api_key: Option<String>) -> Result<serde_json::Value, String> {
    log::info!(
        "[Commands:configure_mano_p_cloud] url={} | key_set={}",
        api_url,
        api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false)
    );

    crate::manop::initialize_mano_p_with_config(&api_url, api_key.as_deref())
        .await
        .map_err(|e| e.to_string())?;

    Ok(serde_json::json!({
        "success": true,
        "mode": "cloud",
        "api_url": api_url,
        "api_key_configured": api_key.as_ref().map(|k| !k.is_empty()).unwrap_or(false),
    }))

}

/// 搜索已安装应用 — 按关键词模糊搜索应用索引
#[tauri::command]
pub async fn search_apps(query: String) -> Result<serde_json::Value, String> {
    log::info!("[Commands:search_apps] query={}", query);
    let results = crate::platform::app_index::search(&query);
    Ok(serde_json::json!({
        "success": true,
        "query": query,
        "count": results.len(),
        "apps": results,
    }))
}

/// 按名称查找应用 — 精确匹配应用名称
#[tauri::command]
pub async fn find_app(name: String) -> Result<serde_json::Value, String> {
    log::info!("[Commands:find_app] name={}", name);
    match crate::platform::app_index::find_by_name(&name) {
        Some(app) => Ok(serde_json::json!({
            "success": true,
            "found": true,
            "app": app,
        })),
        None => Ok(serde_json::json!({
            "success": true,
            "found": false,
            "app": null,
        })),
    }
}

/// 获取所有已索引应用 — 返回完整应用列表
#[tauri::command]
pub async fn get_all_indexed_apps() -> Result<serde_json::Value, String> {
    let apps = crate::platform::app_index::get_all_apps();
    Ok(serde_json::json!({
        "success": true,
        "total": apps.len(),
        "apps": apps,
    }))
}

/// 刷新应用索引 — 强制重新扫描系统应用
#[tauri::command]
pub async fn refresh_app_index() -> Result<serde_json::Value, String> {
    log::info!("[Commands:refresh_app_index] Force refreshing app index");
    match crate::platform::app_index::refresh_index() {
        Ok(apps) => Ok(serde_json::json!({
            "success": true,
            "total": apps.len(),
            "message": format!("Index refreshed with {} apps", apps.len()),
        })),
        Err(e) => Err(format!("Failed to refresh index: {}", e)),
    }
}

/// 获取应用索引统计 — 返回索引应用数量和状态
#[tauri::command]
pub fn get_app_index_stats() -> Result<serde_json::Value, String> {
    let stats = crate::platform::app_index::get_stats();
    Ok(serde_json::json!({
        "success": true,
        "stats": stats,
    }))
}

/// 执行CUA指令 — 通过CUA Agent循环执行桌面自动化
#[tauri::command]
pub async fn execute_cua_instruction(instruction: String) -> Result<serde_json::Value, String> {
    log::info!("[Commands:execute_cua_instruction] instruction={}", instruction);

    let engine_arc = {
        let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        engine_guard.clone()
    };

    match engine_arc {
        Some(engine) => {
            let result = engine.execute_cua_instruction(&instruction).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "success": result.success,
                "instruction": result.instruction,
                "total_steps": result.total_steps,
                "elapsed_ms": result.elapsed_ms,
                "steps": result.steps,
                "error": result.error,
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}

/// OCR识别屏幕 — 捕获屏幕并返回识别的文字内容
#[tauri::command]
pub async fn ocr_recognize_screen(language: Option<String>) -> Result<serde_json::Value, String> {
    log::info!("[Commands:ocr_recognize_screen] language={:?}", language);
    let ocr_text = crate::capture::screen::ocr_screen_text().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "text": ocr_text,
    }))
}

/// 列出已安装应用 — 可选按关键词过滤
#[tauri::command]
pub async fn list_installed_apps(filter: Option<String>) -> Result<serde_json::Value, String> {
    log::info!("[Commands:list_installed_apps] filter={:?}", filter);

    let engine_arc = {
        let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        engine_guard.clone()
    };

    match engine_arc {
        Some(engine) => {
            let apps = engine.list_installed_apps(filter.as_deref()).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "success": true,
                "apps": apps,
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}

/// 启动应用 — 按名称查找并启动
#[tauri::command]
pub async fn launch_application(name: String) -> Result<serde_json::Value, String> {
    log::info!("[Commands:launch_application] name={}", name);

    let engine_arc = {
        let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        engine_guard.clone()
    };

    match engine_arc {
        Some(engine) => {
            engine.launch_application(&name).map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "success": true,
                "message": format!("Launched: {}", name),
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}

/// 截屏OCR — 捕获屏幕并返回识别文字
#[tauri::command]
pub async fn capture_screen() -> Result<String, String> {
    let ocr_text = crate::capture::screen::ocr_screen_text().map_err(|e| e.to_string())?;
    Ok(ocr_text)
}

/// 鼠标左键点击 — 在指定坐标单击
#[tauri::command]
pub async fn mouse_click(x: f64, y: f64) -> Result<(), String> {
    use crate::input::mouse;
    mouse::click(x, y).await.map_err(|e| e.to_string())
}

/// 鼠标左键双击 — 在指定坐标双击
#[tauri::command]
pub async fn mouse_double_click(x: f64, y: f64) -> Result<(), String> {
    use crate::input::mouse;
    mouse::double_click(x, y).await.map_err(|e| e.to_string())
}

/// 键盘输入文本 — 模拟逐字输入
#[tauri::command]
pub async fn keyboard_type(text: String) -> Result<(), String> {
    use crate::input::keyboard;
    keyboard::type_text(&text).await.map_err(|e| e.to_string())
}

/// 键盘按键 — 模拟按下指定按键
#[tauri::command]
pub async fn keyboard_press(key: String) -> Result<(), String> {
    use crate::input::keyboard;
    keyboard::press_key(&key).await.map_err(|e| e.to_string())
}

/// 获取自动化引擎配置 — 返回当前引擎的所有配置项
#[tauri::command]
pub fn get_automation_config() -> Result<serde_json::Value, String> {
    let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
    match engine_guard.as_ref() {
        Some(engine) => {
            let config = engine.config();
            Ok(serde_json::json!({
                "manop_enabled": config.manop_enabled,
                "manop_version": config.manop_version,
                "inference_timeout_secs": config.inference_timeout_secs,
                "max_action_steps": config.max_action_steps,
                "confidence_threshold": config.confidence_threshold,
                "ocr_language": config.ocr_language,
                "llm_model": config.llm_model,
                "llm_api_endpoint": config.llm_api_endpoint,
                "cua_enabled": config.cua_enabled,
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}

/// 获取活动窗口信息 — 返回当前焦点窗口的标题和位置
#[tauri::command]
pub async fn get_active_window() -> Result<serde_json::Value, String> {
    use crate::platform::window;
    let info = window::get_active_window().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "window": info,
    }))
}

/// 获取活动窗口标题
#[tauri::command]
pub async fn get_window_title() -> Result<serde_json::Value, String> {
    use crate::platform::window;
    let title = window::get_window_title().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "title": title,
    }))
}

/// 列出所有可见窗口 — 返回窗口标题和位置列表
#[tauri::command]
pub async fn list_windows() -> Result<serde_json::Value, String> {
    use crate::platform::window;
    let windows = window::list_windows().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "windows": windows,
    }))
}

/// 聚焦窗口 — 按标题关键词查找并激活窗口
#[tauri::command]
pub async fn focus_window(title_contains: String) -> Result<serde_json::Value, String> {
    use crate::platform::window;
    window::focus_window(&title_contains).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "message": format!("Focused window containing '{}'", title_contains),
    }))
}

/// 获取屏幕尺寸 — 返回主显示器的宽高
#[tauri::command]
pub async fn get_screen_size() -> Result<serde_json::Value, String> {
    use crate::platform::window;
    let (width, height) = window::get_screen_size().map_err(|e| e.to_string())?;
    Ok(serde_json::json!({
        "success": true,
        "width": width,
        "height": height,
    }))
}

/// 鼠标滚动 — 正数向上滚动，负数向下滚动
#[tauri::command]
pub async fn mouse_scroll(amount: i32) -> Result<(), String> {
    use crate::input::mouse;
    mouse::scroll(amount).await.map_err(|e| e.to_string())
}

/// 鼠标右键点击 — 在指定坐标右键单击
#[tauri::command]
pub async fn mouse_right_click(x: f64, y: f64) -> Result<(), String> {
    use crate::input::mouse;
    mouse::right_click(x, y).await.map_err(|e| e.to_string())
}

/// 鼠标拖拽 — 从起点拖拽到终点
#[tauri::command]
pub async fn mouse_drag(from_x: f64, from_y: f64, to_x: f64, to_y: f64) -> Result<(), String> {
    use crate::input::mouse;
    mouse::drag(from_x, from_y, to_x, to_y).await.map_err(|e| e.to_string())
}

use crate::manop::{ManoPModelVersion, initialize_mano_p, is_mano_p_ready};
use crate::manop::model_manager::ManoPModelManager;

/// 初始化Mano-P模型 — 加载本地模型或配置云端推理
#[tauri::command]
pub async fn init_mano_p_model() -> Result<serde_json::Value, String> {
    log::info!("[Commands:init_mano_p_model] Initializing Mano-P model");

    match initialize_mano_p().await {
        Ok(_) => {
            let version = if is_mano_p_ready() {
                "quantized_4b"
            } else {
                "unknown"
            };
            Ok(serde_json::json!({
                "success": true,
                "message": "Mano-P model initialized successfully",
                "version": version,
                "status": "ready"
            }))
        }
        Err(e) => {
            log::error!("[Commands:init_mano_p_model] Failed to initialize: {}", e);
            Ok(serde_json::json!({
                "success": false,
                "message": format!("Failed to initialize Mano-P: {}", e),
                "version": null,
                "status": "error"
            }))
        }
    }
}

/// 获取Mano-P状态 — 返回当前模式、版本和云端配置信息
#[tauri::command]
pub async fn get_mano_p_status() -> Result<serde_json::Value, String> {
    let state = crate::manop::get_mano_p_state();
    let state_guard = state.lock().await;

    Ok(serde_json::json!({
        "success": true,
        "mode": state_guard.mode().to_string(),
        "model_version": state_guard.model_version().display_name(),
        "cloud_api_url": state_guard.cloud_manager().cloud_api_url(),
        "cloud_api_key_configured": state_guard.cloud_manager().cloud_api_key().is_some(),
    }))
}

/// 下载Mano-P模型 — 按版本下载模型文件
#[tauri::command]
pub async fn download_mano_p_model(version: Option<String>) -> Result<serde_json::Value, String> {
    let model_version = match version.as_deref() {
        Some("full_72b") => ManoPModelVersion::Full72B,
        _ => ManoPModelVersion::Quantized4B,
    };

    log::info!("[Commands:download_mano_p_model] Downloading {}", model_version.display_name());

    let manager = ManoPModelManager::new();

    match manager.download_model(model_version).await {
        Ok(_) => {
            Ok(serde_json::json!({
                "success": true,
                "message": format!("{} downloaded successfully", model_version.display_name()),
                "version": model_version.model_id(),
            }))
        }
        Err(e) => {
            Err(format!("Failed to download model: {}", e))
        }
    }
}

/// 执行Mano-P指令 — 通过Mano-P推理引擎执行桌面自动化
#[tauri::command]
pub async fn execute_mano_p_instruction(instruction: String) -> Result<serde_json::Value, String> {
    log::info!("[Commands:execute_mano_p_instruction] instruction={}", instruction);

    let engine_arc = {
        let engine_guard = ENGINE.lock().map_err(|e: std::sync::PoisonError<_>| e.to_string())?;
        engine_guard.clone()
    };

    match engine_arc {
        Some(engine) => {
            let result = engine.execute_instruction(&instruction).await
                .map_err(|e| e.to_string())?;
            Ok(serde_json::json!({
                "success": result.success,
                "instruction": result.instruction,
                "intent": result.intent,
                "confidence": result.confidence,
                "inference_time_ms": result.inference_time_ms,
                "action_count": result.action_count,
                "error": result.error_message,
            }))
        }
        None => Err("Automation engine not initialized".to_string()),
    }
}
