// Claw Desktop - Mano-P 桌面操作推理引擎
// 管理Mano-P模型的状态（本地/云端/VLM降级）、初始化、推理任务执行，
// 支持点击/双击/输入/滚动/拖拽/快捷键/窗口操作等桌面动作
pub mod action_executor;
pub mod config;
pub mod inference_engine;
pub mod model_manager;

use crate::error::Result;
use crate::types::{ImageFrame, Point, UiElement};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Mano-P模型版本枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManoPModelVersion {
    Full72B,
    Quantized4B,
}

impl ManoPModelVersion {
    /// 获取模型ID — 用于下载URL路径
    pub fn model_id(&self) -> &'static str {
        match self {
            ManoPModelVersion::Full72B => "Mininglamp-AI/Mano-P-72B",
            ManoPModelVersion::Quantized4B => "Mininglamp-AI/Mano-P-4B-GGUF",
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            ManoPModelVersion::Full72B => "Mano-P 72B",
            ManoPModelVersion::Quantized4B => "Mano-P 4B (Quantized)",
        }
    }

    /// 是否推荐用于桌面自动化 — 仅量化4B版本推荐
    pub fn recommended_for_desktop(&self) -> bool {
        matches!(self, ManoPModelVersion::Quantized4B)
    }
}

/// Mano-P推理结果 — 包含意图、目标元素、动作序列和置信度
#[derive(Debug, Clone)]
pub struct ManoPInferenceResult {
    pub intent: String,
    pub target_elements: Vec<UiElement>,
    pub action_sequence: Vec<ManoPAction>,
    pub confidence: f32,
    pub inference_time_ms: u64,
}

/// Mano-P桌面动作枚举 — 支持点击/双击/右键/输入/滚动/拖拽/等待/截图/快捷键/窗口操作
#[derive(Debug, Clone)]
pub enum ManoPAction {
    Click {
        element_id: String,
        point: Point,
    },
    DoubleClick {
        element_id: String,
        point: Point,
    },
    RightClick {
        element_id: String,
        point: Point,
    },
    TypeText {
        element_id: String,
        text: String,
    },
    Scroll {
        element_id: String,
        direction: ScrollDirection,
        amount: i32,
    },
    Drag {
        from: Point,
        to: Point,
    },
    Wait {
        duration_ms: u64,
    },
    ScreenshotVerify {
        expected_elements: Vec<String>,
    },
    Hotkey {
        keys: Vec<String>,
    },
    WindowAction {
        action: WindowActionType,
    },
}

/// 滚动方向枚举
#[derive(Debug, Clone)]
pub enum ScrollDirection {
    Up,
    Down,
    Left,
    Right,
}

/// 窗口操作类型枚举
#[derive(Debug, Clone)]
pub enum WindowActionType {
    Minimize,
    Maximize,
    Close,
    Focus,
    Move { x: i32, y: i32 },
    Resize { width: i32, height: i32 },
}

/// Mano-P任务请求 — 包含任务描述、截图、历史动作和超时配置
#[derive(Debug, Clone)]
pub struct ManoPTaskRequest {
    pub task_description: String,
    pub screenshot: ImageFrame,
    pub history: Vec<ManoPAction>,
    pub max_steps: Option<usize>,
    pub timeout_secs: Option<u64>,
}

/// Mano-P执行结果 — 包含指令、意图、置信度、推理时间和执行状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManoPExecutionResult {
    pub instruction: String,
    pub intent: String,
    pub confidence: f32,
    pub inference_time_ms: u64,
    pub action_count: usize,
    pub success: bool,
    pub error_message: Option<String>,
}

impl ManoPExecutionResult {
    /// 格式化执行结果摘要
    pub fn format_summary(&self) -> String {
        format!(
            "Mano-P Execution Result:\n\
            - Instruction: {}\n\
            - Intent: {}\n\
            - Confidence: {:.2}\n\
            - Inference Time: {}ms\n\
            - Actions Executed: {}\n\
            - Success: {}\n\
            {}",
            self.instruction,
            self.intent,
            self.confidence,
            self.inference_time_ms,
            self.action_count,
            self.success,
            self.error_message
                .as_ref()
                .map(|e| format!("- Error: {}", e))
                .unwrap_or_default()
        )
    }
}

/// Mano-P运行模式枚举
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ManoPMode {
    Cloud,
    Local,
    VlmFallback,
    Unavailable,
}

impl std::fmt::Display for ManoPMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ManoPMode::Cloud => write!(f, "cloud"),
            ManoPMode::Local => write!(f, "local"),
            ManoPMode::VlmFallback => write!(f, "vlm_fallback"),
            ManoPMode::Unavailable => write!(f, "unavailable"),
        }
    }
}

/// Mano-P状态管理 — 管理模型版本、运行模式和推理引擎
pub struct ManoPState {
    model_version: ManoPModelVersion,
    mode: ManoPMode,
    inference_engine: Option<Arc<Mutex<inference_engine::ManoPInferenceEngine>>>,
    cloud_manager: model_manager::ManoPModelManager,
}

impl ManoPState {
    /// 创建Mano-P状态 — 根据平台支持情况选择初始模式
    pub fn new() -> Self {
        let local_support = model_manager::ManoPModelManager::check_local_model_support();
        let mode = if local_support.supported {
            ManoPMode::Local
        } else {
            ManoPMode::Cloud
        };

        log::info!(
            "[ManoP] Platform: {} | Local model: {} | Recommended: {} | Initial mode: {}",
            local_support.platform,
            local_support.supported,
            local_support.recommended_mode,
            mode
        );

        Self {
            model_version: ManoPModelVersion::Quantized4B,
            mode,
            inference_engine: None,
            cloud_manager: model_manager::ManoPModelManager::new(),
        }
    }

    /// 获取当前运行模式
    pub fn mode(&self) -> ManoPMode {
        self.mode
    }

    /// 检查模型是否已加载 — 云端模式始终返回true
    pub fn is_model_loaded(&self) -> bool {
        self.mode == ManoPMode::Cloud || self.inference_engine.is_some()
    }

    /// 获取模型版本
    pub fn model_version(&self) -> ManoPModelVersion {
        self.model_version
    }

    /// 设置模型版本
    pub fn set_model_version(&mut self, version: ManoPModelVersion) {
        self.model_version = version;
    }

    /// 获取云端模型管理器引用
    pub fn cloud_manager(&self) -> &model_manager::ManoPModelManager {
        &self.cloud_manager
    }

    /// 获取云端模型管理器可变引用
    pub fn cloud_manager_mut(&mut self) -> &mut model_manager::ManoPModelManager {
        &mut self.cloud_manager
    }

    /// 设置推理引擎 — 同时切换为本地模式
    pub fn set_inference_engine(&mut self, engine: inference_engine::ManoPInferenceEngine) {
        self.inference_engine = Some(Arc::new(Mutex::new(engine)));
        self.mode = ManoPMode::Local;
    }

    /// 获取推理引擎Arc引用
    pub fn get_inference_engine(
        &self,
    ) -> Option<Arc<Mutex<inference_engine::ManoPInferenceEngine>>> {
        self.inference_engine.clone()
    }

    /// 切换为云端模式
    pub fn set_cloud_mode(&mut self) {
        self.mode = ManoPMode::Cloud;
        log::info!("[ManoP] Switched to cloud mode");
    }

    /// 切换为VLM降级模式
    pub fn set_vlm_fallback_mode(&mut self) {
        self.mode = ManoPMode::VlmFallback;
        log::info!("[ManoP] Switched to VLM fallback mode");
    }
}

impl Default for ManoPState {
    fn default() -> Self {
        Self::new()
    }
}

use std::sync::OnceLock;
static MANO_P_STATE: OnceLock<Arc<tokio::sync::Mutex<ManoPState>>> = OnceLock::new();

/// 获取Mano-P全局状态 — 首次调用时初始化
pub fn get_mano_p_state() -> Arc<tokio::sync::Mutex<ManoPState>> {
    MANO_P_STATE
        .get_or_init(|| Arc::new(tokio::sync::Mutex::new(ManoPState::new())))
        .clone()
}

/// 初始化Mano-P — 检测平台支持，优先本地模型，回退到云端模式
pub async fn initialize_mano_p() -> Result<()> {
    log::info!("[ManoP:initialize] Starting Mano-P initialization");

    let state = get_mano_p_state();
    let mut state_guard = state.lock().await;

    let local_support = model_manager::ManoPModelManager::check_local_model_support();

    if local_support.supported {
        let model_manager = model_manager::ManoPModelManager::new();
        if model_manager
            .is_model_complete(state_guard.model_version())
            .await
        {
            log::info!("[ManoP:initialize] Local model found, creating inference engine");
            match inference_engine::ManoPInferenceEngine::new(state_guard.model_version()).await {
                Ok(engine) => {
                    state_guard.set_inference_engine(engine);
                    log::info!("[ManoP:initialize] Local model loaded successfully");
                    return Ok(());
                }
                Err(e) => {
                    log::warn!(
                        "[ManoP:initialize] Local model engine creation failed: {}, falling back to cloud",
                        e
                    );
                    state_guard.set_cloud_mode();
                    return Ok(());
                }
            }
        } else {
            log::info!(
                "[ManoP:initialize] Local model not downloaded yet. \
                 Mano-P weights are not yet publicly available (Phase 2). \
                 Using cloud mode. See: https://github.com/Mininglamp-AI/Mano-P/issues/2"
            );
            state_guard.set_cloud_mode();
            return Ok(());
        }
    }

    log::info!(
        "[ManoP:initialize] Platform '{}' does not support local Mano-P model ({}). \
         Using cloud mode (mano.mininglamp.com).",
        local_support.platform,
        local_support.reason
    );
    state_guard.set_cloud_mode();

    log::info!(
        "[ManoP:initialize] Initialization completed (mode: {})",
        state_guard.mode()
    );
    Ok(())
}

/// 使用云端配置初始化Mano-P — 设置API地址和密钥，切换为云端模式
pub async fn initialize_mano_p_with_config(
    cloud_api_url: &str,
    cloud_api_key: Option<&str>,
) -> Result<()> {
    log::info!(
        "[ManoP:initialize_with_config] Starting Mano-P initialization with cloud config | url={} | key={}",
        cloud_api_url,
        cloud_api_key
            .map(|k| format!(
                "{}...{}",
                &k[..k.len().min(4)],
                &k[k.len().saturating_sub(4)..]
            ))
            .unwrap_or_else(|| "None".to_string())
    );

    let state = get_mano_p_state();
    let mut state_guard = state.lock().await;

    state_guard
        .cloud_manager_mut()
        .set_cloud_api_url(cloud_api_url.to_string());
    if let Some(key) = cloud_api_key {
        if !key.is_empty() {
            state_guard
                .cloud_manager_mut()
                .set_cloud_api_key(key.to_string());
        }
    }

    state_guard.set_cloud_mode();

    log::info!(
        "[ManoP:initialize_with_config] Cloud mode configured | url={} | key_set={}",
        cloud_api_url,
        cloud_api_key.is_some() && !cloud_api_key.unwrap_or_default().is_empty()
    );

    Ok(())
}

/// 检查Mano-P是否已就绪
pub fn is_mano_p_ready() -> bool {
    match MANO_P_STATE.get() {
        Some(_) => true,
        None => false,
    }
}

/// 执行Mano-P推理任务 — 根据当前模式选择云端/本地/VLM降级推理
pub async fn execute_task(request: ManoPTaskRequest) -> Result<ManoPInferenceResult> {
    if !is_mano_p_ready() {
        initialize_mano_p().await?;
    }

    log::info!(
        "[ManoP:execute_task] Executing task: {} (max_steps: {:?}, timeout: {:?})",
        request.task_description,
        request.max_steps,
        request.timeout_secs
    );

    let state = get_mano_p_state();
    let state_guard = state.lock().await;
    let current_mode = state_guard.mode();

    match current_mode {
        ManoPMode::Cloud => {
            drop(state_guard);
            execute_cloud_task(&request).await
        }
        ManoPMode::Local => match state_guard.get_inference_engine() {
            Some(engine_arc) => {
                drop(state_guard);
                let engine = engine_arc.lock().await;
                engine.infer(&request).await
            }
            None => {
                drop(state_guard);
                log::warn!(
                    "[ManoP:execute_task] Local engine not available, falling back to cloud"
                );
                execute_cloud_task(&request).await
            }
        },
        ManoPMode::VlmFallback => match state_guard.get_inference_engine() {
            Some(engine_arc) => {
                drop(state_guard);
                let engine = engine_arc.lock().await;
                engine.infer(&request).await
            }
            None => {
                drop(state_guard);
                log::warn!("[ManoP:execute_task] No engine available, returning mock result");
                Ok(ManoPInferenceResult {
                    intent: request.task_description.clone(),
                    target_elements: Vec::new(),
                    action_sequence: vec![ManoPAction::ScreenshotVerify {
                        expected_elements: vec!["desktop".to_string()],
                    }],
                    confidence: 0.3,
                    inference_time_ms: 10,
                })
            }
        },
        ManoPMode::Unavailable => Err(crate::error::AutomaticallyError::ManoP(
            "Mano-P is not available. No local model, cloud API, or VLM fallback configured."
                .to_string(),
        )),
    }
}

/// 执行云端推理任务 — 调用Mano云端API，失败时回退到VLM
async fn execute_cloud_task(request: &ManoPTaskRequest) -> Result<ManoPInferenceResult> {
    let start_time = std::time::Instant::now();

    let state = get_mano_p_state();
    let state_guard = state.lock().await;
    let cloud_mgr = state_guard.cloud_manager();

    let screenshot_b64 = request.screenshot.to_base64();

    match cloud_mgr
        .cloud_inference(&request.task_description, &screenshot_b64, None)
        .await
    {
        Ok(cloud_resp) => {
            let inference_time_ms = start_time.elapsed().as_millis() as u64;

            if !cloud_resp.success {
                return Err(crate::error::AutomaticallyError::ManoP(format!(
                    "Cloud inference failed: {}",
                    cloud_resp
                        .error
                        .unwrap_or_else(|| "Unknown error".to_string())
                )));
            }

            let action = parse_cloud_action(&cloud_resp.action_type, &cloud_resp.parameters);

            Ok(ManoPInferenceResult {
                intent: request.task_description.clone(),
                target_elements: Vec::new(),
                action_sequence: vec![action],
                confidence: cloud_resp.confidence as f32,
                inference_time_ms,
            })
        }
        Err(e) => {
            log::warn!(
                "[ManoP:execute_cloud_task] Cloud inference failed: {}, trying VLM fallback",
                e
            );
            drop(state_guard);

            if let Some(_caller) = claw_traits::llm_caller::get_llm_caller() {
                log::info!("[ManoP:execute_cloud_task] Using VLM caller as fallback");
                let engine =
                    inference_engine::ManoPInferenceEngine::new(ManoPModelVersion::Quantized4B)
                        .await?;
                engine.infer(request).await
            } else {
                Err(e)
            }
        }
    }
}

/// 解析云端动作响应 — 将JSON参数转换为ManoPAction枚举
fn parse_cloud_action(action_type: &str, params: &serde_json::Value) -> ManoPAction {
    match action_type {
        "click" => {
            let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(500) as i32;
            let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
            ManoPAction::Click {
                element_id: format!("cloud_point_{}_{}", x, y),
                point: Point { x, y },
            }
        }
        "double_click" => {
            let x = params.get("x").and_then(|v| v.as_i64()).unwrap_or(500) as i32;
            let y = params.get("y").and_then(|v| v.as_i64()).unwrap_or(300) as i32;
            ManoPAction::DoubleClick {
                element_id: format!("cloud_point_{}_{}", x, y),
                point: Point { x, y },
            }
        }
        "type" | "type_text" => {
            let text = params
                .get("text")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            ManoPAction::TypeText {
                element_id: "cloud_input".to_string(),
                text,
            }
        }
        "scroll" => {
            let direction = params
                .get("direction")
                .and_then(|v| v.as_str())
                .unwrap_or("down");
            let amount = params.get("amount").and_then(|v| v.as_i64()).unwrap_or(3) as i32;
            ManoPAction::Scroll {
                element_id: "cloud_scroll".to_string(),
                direction: match direction {
                    "up" => ScrollDirection::Up,
                    "left" => ScrollDirection::Left,
                    "right" => ScrollDirection::Right,
                    _ => ScrollDirection::Down,
                },
                amount,
            }
        }
        "hotkey" => {
            let keys = params
                .get("keys")
                .and_then(|v| v.as_array())
                .map(|arr| {
                    arr.iter()
                        .filter_map(|v| v.as_str().map(String::from))
                        .collect()
                })
                .unwrap_or_default();
            ManoPAction::Hotkey { keys }
        }
        "wait" => {
            let ms = params.get("ms").and_then(|v| v.as_u64()).unwrap_or(1000);
            ManoPAction::Wait { duration_ms: ms }
        }
        _ => {
            log::warn!(
                "[ManoP:parse_cloud_action] Unknown action type: {}, defaulting to screenshot verify",
                action_type
            );
            ManoPAction::ScreenshotVerify {
                expected_elements: Vec::new(),
            }
        }
    }
}
