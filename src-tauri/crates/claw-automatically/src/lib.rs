// Claw Desktop - 桌面自动化引擎
// 提供屏幕捕获、鼠标键盘输入、Mano-P推理、CUA Agent、应用启动等桌面自动化能力
pub mod capture;
pub mod error;
pub mod input;
pub mod manop;
pub mod types;
pub mod platform;
pub mod agent;

pub mod commands;

pub use error::{AutomaticallyError, Result};
pub use types::*;

/// 桌面自动化配置 — 控制截图帧率、Mano-P模型、CUA Agent和LLM参数
#[derive(Debug, Clone)]
pub struct AutomaticallyConfig {
    pub screen_capture_fps: u32,
    pub session_ttl_seconds: i64,
    pub manop_enabled: bool,
    pub manop_version: String,
    pub manop_auto_download: bool,
    pub manop_auto_initialize: bool,
    pub manop_cloud_api_url: String,
    pub manop_cloud_api_key: Option<String>,
    pub inference_timeout_secs: u64,
    pub max_action_steps: usize,
    pub confidence_threshold: f32,
    pub ocr_language: String,
    pub llm_api_endpoint: String,
    pub llm_api_key: Option<String>,
    pub llm_model: String,
    pub cua_enabled: bool,
}

/// 自动化配置默认值 — 30fps截图，Mano-P量化4B，CUA启用
impl Default for AutomaticallyConfig {
    fn default() -> Self {
        Self {
            screen_capture_fps: 30,
            session_ttl_seconds: 7200,
            manop_enabled: true,
            manop_version: "quantized_4b".to_string(),
            manop_auto_download: true,
            manop_auto_initialize: true,
            manop_cloud_api_url: "https://mano.mininglamp.com".to_string(),
            manop_cloud_api_key: None,
            inference_timeout_secs: 30,
            max_action_steps: 50,
            confidence_threshold: 0.75,
            ocr_language: "chi_sim+eng".to_string(),
            llm_api_endpoint: "https://api.openai.com/v1/chat/completions".to_string(),
            llm_api_key: None,
            llm_model: "gpt-4o".to_string(),
            cua_enabled: true,
        }
    }
}

/// 桌面自动化引擎 — 统一封装截图、输入、推理和应用管理能力
pub struct AutomaticallyEngine {
    config: AutomaticallyConfig,
}

impl AutomaticallyEngine {
    /// 创建自动化引擎 — 使用指定配置
    pub fn new(config: AutomaticallyConfig) -> Self {
        Self { config }
    }

    /// 尝试直接启动应用 — 解析指令中的打开/启动关键词，匹配应用索引
    fn try_direct_app_launch(instruction: &str) -> Option<AppInfo> {
        let lower = instruction.to_lowercase();
        let open_patterns = [
            "打开", "打开应用", "启动", "启动应用", "运行",
            "launch", "open", "start", "run",
            "帮我打开", "帮我启动", "请打开",
            "help me open", "please open", "帮我点击打开",
            "click to open", "click open", "点击打开",
        ];

        let mut query = String::new();
        let mut matched = false;
        let mut last_pattern_end = 0usize;

        for pattern in &open_patterns {
            if let Some(pos) = lower.find(pattern) {
                let end_pos = pos + pattern.len();
                if end_pos > last_pattern_end {
                    last_pattern_end = end_pos;
                    matched = true;
                }
            }
        }

        if matched {
            let rest = instruction[last_pattern_end..].trim();
            let search_region = &instruction[..last_pattern_end];
            let split_pos = search_region.rfind(|c: char| !c.is_ascii_alphabetic() && c != ' ').unwrap_or(0);
            let before = instruction[split_pos..last_pattern_end].trim();

            if !rest.is_empty() && (rest.len() < before.len() || before.is_empty()) {
                query = rest.to_string();
            } else if !before.is_empty() {
                query = before.to_string();
            } else {
                query = rest.to_string();
            }

            query = query.trim_matches(|c: char| c == ',' || c == '，' || c == '.' || c == '。').to_string();
        }

        if !matched {
            let app_keywords = ["qclaw", "微信", "wechat", "chrome", "notepad", "explorer", "vscode",
                "word", "excel", "powerpoint", "outlook", "firefox", "edge",
                "slack", "discord", "telegram", "spotify", "vlc", "steam",
                "qq", "tim", "飞书", "feishu", "钉钉", "dingtalk"];
            for kw in &app_keywords {
                if lower.contains(kw) {
                    query = instruction.trim().to_string();
                    matched = true;
                    break;
                }
            }
        }

        if !matched || query.is_empty() || query.len() < 2 {
            return None;
        }

        log::info!("[AutomaticallyEngine:try_direct_app_launch] instruction='{}' → extracted='{}'", instruction, query);

        let best_match = platform::app_index::find_best_match(&query);
        if let Some(app) = best_match {
            log::info!("[AutomaticallyEngine:try_direct_app_launch] ✅ Found '{}' (source: {}, path: {})", app.name, app.app_source, app.executable_path);
            if let Err(e) = platform::app_launcher::launch_application(&app.name) {
                log::warn!("[AutomaticallyEngine:try_direct_app_launch] ❌ Launch by name failed: {}, trying launch_command", e);
                if let Some(ref cmd) = app.launch_command {
                    if !cmd.is_empty() {
                        if let Err(e2) = platform::app_launcher::launch_application(cmd) {
                            log::warn!("[AutomaticallyEngine:try_direct_app_launch] ❌ Launch by command also failed: {}", e2);
                            return None;
                        }
                    } else {
                        return None;
                    }
                } else {
                    return None;
                }
            }
            return Some(app);
        }

        let search_results = platform::app_index::search(&query);
        if let Some(app) = search_results.first() {
            log::info!("[AutomaticallyEngine:try_direct_app_launch] ✅ Fuzzy match found '{}' (source: {}, score: relevant)", app.name, app.app_source);
            let launch_target = if !app.executable_path.is_empty() && std::path::Path::new(&app.executable_path).exists() {
                &app.executable_path
            } else if let Some(ref cmd) = app.launch_command {
                cmd
            } else {
                &app.name
            };
            if let Err(e) = platform::app_launcher::launch_application(launch_target) {
                log::warn!("[AutomaticallyEngine:try_direct_app_launch] ❌ Fuzzy match launch failed: {}", e);
                return None;
            }
            return Some(app.clone());
        }

        log::info!("[AutomaticallyEngine:try_direct_app_launch] No match for '{}', trying launch_application directly", query);
        match platform::app_launcher::launch_application(&query) {
            Ok(()) => {
                log::info!("[AutomaticallyEngine:try_direct_app_launch] ✅ Direct launch succeeded for '{}'", query);
                Some(AppInfo {
                    name: query.clone(),
                    executable_path: String::new(),
                    description: None,
                    publisher: None,
                    version: None,
                    launch_command: Some(query),
                    app_source: "direct_launch".to_string(),
                    keywords: Vec::new(),
                })
            }
            Err(e) => {
                log::warn!("[AutomaticallyEngine:try_direct_app_launch] ❌ Direct launch also failed: {}", e);
                None
            }
        }
    }

    /// 获取引擎配置引用
    pub fn config(&self) -> &AutomaticallyConfig {
        &self.config
    }

    /// 判断指令是否为简单的应用启动（只需打开应用，无需后续交互）
    fn is_simple_launch(instruction: &str) -> bool {
        let lower = instruction.to_lowercase();
        let simple_patterns = [
            "打开", "启动", "运行", "launch", "open", "start", "run",
            "帮我打开", "帮我启动", "请打开", "help me open", "please open",
        ];
        let complex_keywords = [
            "发消息", "发送", "输入", "点击", "搜索", "查找", "登录", "扫码",
            "填写", "选择", "切换", "复制", "粘贴", "拖动", "截图", "截图识别",
            "send", "type", "click", "search", "find", "login", "fill",
            "select", "switch", "copy", "paste", "drag", "scroll",
            "给", "跟", "和", "聊天", "对话", "消息", "文件", "打开后",
            "然后", "接着", "再", "之后", "并且", "同时",
        ];

        let is_launch = simple_patterns.iter().any(|p| lower.contains(p));
        let has_complex = complex_keywords.iter().any(|k| lower.contains(k));

        is_launch && !has_complex
    }

    /// 执行自动化指令 — 复杂交互走CUA，简单启动走直接启动，Mano-P作为备选
    pub async fn execute_instruction(&self, instruction: &str) -> Result<manop::ManoPExecutionResult> {
        log::info!("[AutomaticallyEngine:execute_instruction] instruction={}", instruction);

        if !Self::is_simple_launch(instruction) && self.config.cua_enabled && self.config.llm_api_key.is_some() {
            log::info!("[AutomaticallyEngine:execute_instruction] Complex task detected, using CUA Agent mode");
            return self.execute_cua(instruction).await;
        }

        if let Some(app) = Self::try_direct_app_launch(instruction) {
            log::info!("[AutomaticallyEngine:execute_instruction] Direct app launch matched: {}", app.name);

            if !Self::is_simple_launch(instruction) && self.config.cua_enabled && self.config.llm_api_key.is_none() {
                log::warn!("[AutomaticallyEngine:execute_instruction] Complex task but no LLM API key for CUA, task may be incomplete");
            }

            return Ok(manop::ManoPExecutionResult {
                instruction: instruction.to_string(),
                intent: format!("launch_app:{}", app.name),
                confidence: 0.99,
                inference_time_ms: 1,
                action_count: 1,
                success: true,
                error_message: None,
            });
        }

        if self.config.cua_enabled && self.config.llm_api_key.is_some() {
            log::info!("[AutomaticallyEngine:execute_instruction] No direct launch match, using CUA Agent mode");
            return self.execute_cua(instruction).await;
        }

        if !self.config.manop_enabled {
            return Err(AutomaticallyError::ManoP(
                "Neither CUA nor Mano-P is available. Please configure an LLM API key in Settings to enable CUA Agent.".to_string()
            ));
        }

        match manop::initialize_mano_p().await {
            Ok(_) => {
                let cloud_url = self.config.manop_cloud_api_url.clone();
                let cloud_key = self.config.manop_cloud_api_key.clone();
                if let Err(e) = manop::initialize_mano_p_with_config(&cloud_url, cloud_key.as_deref()).await {
                    log::warn!("[AutomaticallyEngine:execute_instruction] Cloud config apply failed: {}", e);
                }
            }
            Err(e) => {
                log::warn!("[AutomaticallyEngine:execute_instruction] Mano-P init failed ({}), trying CUA fallback", e);
                if self.config.llm_api_key.is_some() {
                    log::info!("[AutomaticallyEngine:execute_instruction] Falling back to CUA Agent");
                    return self.execute_cua(instruction).await;
                }
                return Err(AutomaticallyError::ManoP(format!(
                    "Mano-P initialization failed and no LLM API key configured for CUA fallback: {}", e
                )));
            }
        }

        let screenshot = capture::screen::capture_screen()?;
        let request = manop::ManoPTaskRequest {
            task_description: instruction.to_string(),
            screenshot,
            history: Vec::new(),
            max_steps: Some(self.config.max_action_steps),
            timeout_secs: Some(self.config.inference_timeout_secs),
        };

        let inference_result = manop::execute_task(request).await?;

        if inference_result.confidence < self.config.confidence_threshold {
            log::warn!(
                "[AutomaticallyEngine:execute_instruction] Low confidence: {:.2} < threshold {:.2}",
                inference_result.confidence,
                self.config.confidence_threshold
            );
        }

        let mut executor = manop::action_executor::ActionExecutor::new();
        let execution_context = executor
            .execute_sequence(inference_result.action_sequence.clone())
            .await?;

        Ok(manop::ManoPExecutionResult {
            instruction: instruction.to_string(),
            intent: inference_result.intent,
            confidence: inference_result.confidence,
            inference_time_ms: inference_result.inference_time_ms,
            action_count: execution_context.step_count,
            success: execution_context.success,
            error_message: execution_context.error_message,
        })
    }

    /// 执行CUA Agent — 通过截图+LLM循环执行桌面自动化，自动匹配应用技能
    async fn execute_cua(&self, instruction: &str) -> Result<manop::ManoPExecutionResult> {
        let cua_agent = agent::cua_agent::CuaAgent::new_with_skill(self.config.clone(), instruction);
        let result = cua_agent.execute(instruction).await?;

        Ok(manop::ManoPExecutionResult {
            instruction: result.instruction,
            intent: instruction.to_string(),
            confidence: if result.success { 0.95 } else { 0.5 },
            inference_time_ms: result.elapsed_ms,
            action_count: result.total_steps,
            success: result.success,
            error_message: result.error,
        })
    }

    /// 执行CUA指令 — 返回原始CUA执行结果，自动匹配应用技能
    pub async fn execute_cua_instruction(&self, instruction: &str) -> Result<agent::cua_agent::CuaExecutionResult> {
        let cua_agent = agent::cua_agent::CuaAgent::new_with_skill(self.config.clone(), instruction);
        cua_agent.execute(instruction).await
    }

    /// 截取屏幕 — 返回当前屏幕的图像帧
    pub fn capture_screen(&self) -> Result<ImageFrame> {
        capture::screen::capture_screen()
    }

    /// 鼠标左键点击
    pub async fn mouse_click(&self, x: f64, y: f64) -> Result<()> {
        input::mouse::click(x, y).await
    }

    /// 鼠标左键双击
    pub async fn mouse_double_click(&self, x: f64, y: f64) -> Result<()> {
        input::mouse::double_click(x, y).await
    }

    /// 鼠标右键点击
    pub async fn mouse_right_click(&self, x: f64, y: f64) -> Result<()> {
        input::mouse::right_click(x, y).await
    }

    /// 键盘输入文本
    pub async fn keyboard_type(&self, text: &str) -> Result<()> {
        input::keyboard::type_text(text).await
    }

    /// 键盘按键
    pub async fn keyboard_press(&self, key: &str) -> Result<()> {
        input::keyboard::press_key(key).await
    }

    /// 获取活动窗口信息
    pub fn get_active_window(&self) -> Result<WindowInfo> {
        platform::window::get_active_window()
    }

    /// 获取活动窗口标题
    pub fn get_window_title(&self) -> Result<String> {
        platform::window::get_window_title()
    }

    /// 列出所有可见窗口
    pub fn list_windows(&self) -> Result<Vec<WindowInfo>> {
        platform::window::list_windows()
    }

    /// 聚焦窗口 — 按标题关键词查找并激活
    pub fn focus_window(&self, title_contains: &str) -> Result<()> {
        platform::window::focus_window(title_contains)
    }

    /// 获取屏幕尺寸
    pub fn get_screen_size(&self) -> Result<(u32, u32)> {
        platform::window::get_screen_size()
    }

    /// 启动应用 — 按名称查找并启动
    pub fn launch_application(&self, name: &str) -> Result<()> {
        platform::app_launcher::launch_application(name)
    }

    /// 列出已安装应用 — 可选按关键词过滤
    pub fn list_installed_apps(&self, filter: Option<&str>) -> Result<Vec<AppInfo>> {
        platform::app_launcher::list_installed_apps(filter)
    }

    /// 搜索应用 — 按关键词模糊搜索
    pub fn search_apps(&self, query: &str) -> Vec<AppInfo> {
        log::info!("[AutomaticallyEngine:search_apps] query={}", query);
        platform::app_index::search(query)
    }

    /// 按名称查找应用
    pub fn find_app(&self, name: &str) -> Option<AppInfo> {
        log::info!("[AutomaticallyEngine:find_app] name={}", name);
        platform::app_index::find_by_name(name)
    }

    /// 获取所有已索引应用
    pub fn get_all_indexed_apps(&self) -> Vec<AppInfo> {
        platform::app_index::get_all_apps()
    }

    /// 刷新应用索引 — 强制重新扫描系统应用
    pub fn refresh_app_index(&self) -> Result<Vec<AppInfo>> {
        log::info!("[AutomaticallyEngine:refresh_app_index] Refreshing app index");
        platform::app_index::refresh_index()
    }

    /// 获取应用索引统计信息
    pub fn get_app_index_stats(&self) -> serde_json::Value {
        platform::app_index::get_stats()
    }
}
