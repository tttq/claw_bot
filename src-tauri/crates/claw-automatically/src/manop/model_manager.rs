// Claw Desktop - Mano-P 模型管理器
// 负责模型下载、本地模型完整性检查、云端推理API调用、平台兼容性检测
use super::ManoPModelVersion;
use crate::error::{AutomaticallyError, Result};
use std::path::{Path, PathBuf};
use tokio::fs;
use tokio::io::AsyncWriteExt;

const MANO_CLOUD_BASE_URL: &str = "https://mano.mininglamp.com";
const MANO_CLIENT_VERSION: &str = "1.0.8";

/// 模型下载源 — 记录来源名称、基础URL和优先级
#[derive(Debug, Clone)]
pub struct ModelSource {
    pub name: &'static str,
    pub base_url: &'static str,
    pub priority: u8,
}

const DEFAULT_SOURCES: &[ModelSource] = &[
    ModelSource {
        name: "HuggingFace",
        base_url: "https://huggingface.co/Mininglamp-AI",
        priority: 1,
    },
    ModelSource {
        name: "ModelScope",
        base_url: "https://www.modelscope.cn/models/Mininglamp-AI",
        priority: 2,
    },
    ModelSource {
        name: "GitHub Releases",
        base_url: "https://github.com/Mininglamp-AI/Mano-P/releases/download",
        priority: 3,
    },
];

const MODEL_4B_FILES: &[&str] = &[
    "Mano-P-4B-Q4_K_M.gguf",
    "tokenizer.json",
    "tokenizer_config.json",
    "config.json",
];

const MODEL_72B_FILES: &[&str] = &[
    "model-00001-of-00010.safetensors",
    "model-00002-of-00010.safetensors",
    "model-00003-of-00010.safetensors",
    "model-00004-of-00010.safetensors",
    "model-00005-of-00010.safetensors",
    "model-00006-of-00010.safetensors",
    "model-00007-of-00010.safetensors",
    "model-00008-of-00010.safetensors",
    "model-00009-of-00010.safetensors",
    "model-00010-of-00010.safetensors",
    "config.json",
    "tokenizer.json",
    "tokenizer_config.json",
    "model.safetensors.index.json",
];

/// 模型元数据 — 记录版本、总大小、文件数和下载URL模板
#[derive(Debug, Clone)]
pub struct ModelMetadata {
    pub version: ManoPModelVersion,
    pub total_size_bytes: u64,
    pub file_count: usize,
    pub download_url_template: String,
}

impl ModelMetadata {
    /// 根据版本创建模型元数据 — 包含文件大小、数量和下载模板
    pub fn for_version(version: ManoPModelVersion) -> Self {
        match version {
            ManoPModelVersion::Full72B => Self {
                version,
                total_size_bytes: 144_000_000_000,
                file_count: MODEL_72B_FILES.len(),
                download_url_template: "{base}/{model_id}/resolve/main/{filename}".to_string(),
            },
            ManoPModelVersion::Quantized4B => Self {
                version,
                total_size_bytes: 2_500_000_000,
                file_count: MODEL_4B_FILES.len(),
                download_url_template: "{base}/{model_id}/resolve/main/{filename}".to_string(),
            },
        }
    }
}

/// Mano云端推理响应 — 包含动作、置信度和模型信息
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManoCloudResponse {
    pub success: bool,
    pub action: String,
    pub action_type: String,
    pub parameters: serde_json::Value,
    pub confidence: f64,
    pub model_used: String,
    pub error: Option<String>,
}

/// Mano云端会话响应 — 返回会话ID
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManoSessionResponse {
    pub session_id: String,
}

/// Mano云端步骤响应 — 包含动作列表、推理过程和状态
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ManoStepResponse {
    pub actions: Vec<serde_json::Value>,
    pub reasoning: Option<String>,
    pub action_desc: Option<String>,
    pub status: Option<String>,
}

/// 构建HTTP User-Agent头 — 包含客户端版本、操作系统和架构信息
fn build_user_agent() -> String {
    let os = std::env::consts::OS;
    let arch = std::env::consts::ARCH;
    format!("mano-cua/{} ({}; {}) Rust", MANO_CLIENT_VERSION, os, arch)
}

/// 获取设备唯一ID — 首次生成UUID并持久化到本地文件
fn get_device_id() -> String {
    let id_file = dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("claw-desktop")
        .join(".device_id");

    if let Ok(id) = std::fs::read_to_string(&id_file) {
        let trimmed = id.trim().to_string();
        if !trimmed.is_empty() {
            return trimmed;
        }
    }

    let new_id = uuid::Uuid::new_v4().to_string();
    if let Some(parent) = id_file.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&id_file, &new_id);
    new_id
}

/// Mano-P模型管理器 — 负责模型下载、完整性检查和云端推理API调用
pub struct ManoPModelManager {
    model_dir: PathBuf,
    sources: Vec<ModelSource>,
    cloud_api_key: Option<String>,
    cloud_api_url: String,
}

impl ManoPModelManager {
    /// 创建模型管理器 — 使用默认模型目录和下载源
    pub fn new() -> Self {
        let model_dir = Self::default_model_dir();
        Self {
            model_dir,
            sources: DEFAULT_SOURCES.to_vec(),
            cloud_api_key: None,
            cloud_api_url: MANO_CLOUD_BASE_URL.to_string(),
        }
    }

    /// 创建模型管理器 — 使用指定模型目录
    pub fn with_model_dir(model_dir: PathBuf) -> Self {
        Self {
            model_dir,
            sources: DEFAULT_SOURCES.to_vec(),
            cloud_api_key: None,
            cloud_api_url: MANO_CLOUD_BASE_URL.to_string(),
        }
    }

    /// 设置云端API密钥 — Builder模式
    pub fn with_cloud_api_key(mut self, key: String) -> Self {
        self.cloud_api_key = Some(key);
        self
    }

    /// 设置云端API地址 — Builder模式
    pub fn with_cloud_api_url(mut self, url: String) -> Self {
        self.cloud_api_url = url;
        self
    }

    /// 获取默认模型目录路径 — ~/AppData/Local/claw-desktop/models/mano-p
    fn default_model_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("claw-desktop")
            .join("models")
            .join("mano-p")
    }

    /// 获取云端API地址
    pub fn cloud_api_url(&self) -> &str {
        &self.cloud_api_url
    }

    /// 获取云端API密钥
    pub fn cloud_api_key(&self) -> Option<&str> {
        self.cloud_api_key.as_deref()
    }

    /// 设置云端API密钥
    pub fn set_cloud_api_key(&mut self, key: String) {
        self.cloud_api_key = Some(key);
    }

    /// 设置云端API地址
    pub fn set_cloud_api_url(&mut self, url: String) {
        self.cloud_api_url = url;
    }

    /// 云端推理 — 创建会话→发送截图→获取动作结果→自动关闭会话
    pub async fn cloud_inference(
        &self,
        task: &str,
        screenshot_b64: &str,
        model_preference: Option<&str>,
    ) -> Result<ManoCloudResponse> {
        log::info!(
            "[ManoPModelManager:cloud_inference] Creating session for task: '{}' (screenshot {} bytes)",
            task,
            screenshot_b64.len()
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(600))
            .build()
            .map_err(|e| {
                AutomaticallyError::ManoP(format!("HTTP client creation failed: {}", e))
            })?;

        let device_id = get_device_id();
        let platform = std::env::consts::OS.to_string();
        let user_agent = build_user_agent();

        let mut session_body = serde_json::json!({
            "device_id": device_id,
            "platform": platform,
            "task": task,
        });
        if let Some(pref) = model_preference {
            session_body["model_preference"] = serde_json::json!(pref);
        }

        let session_resp = client
            .post(format!("{}/v1/sessions", self.cloud_api_url))
            .header("User-Agent", &user_agent)
            .json(&session_body)
            .send()
            .await
            .map_err(|e| {
                log::error!(
                    "[ManoPModelManager:cloud_inference] Session creation failed: {}",
                    e
                );
                AutomaticallyError::ManoP(format!("Mano cloud session creation failed: {}", e))
            })?;

        let session_status = session_resp.status();
        let session_text = session_resp.text().await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to read session response: {}", e))
        })?;

        if session_status.as_u16() == 409 {
            return Err(AutomaticallyError::ManoP(
                "Another task is already running on this device. Stop it first.".to_string(),
            ));
        }

        if !session_status.is_success() {
            return Err(AutomaticallyError::ManoP(format!(
                "Mano cloud session error (HTTP {}): {}",
                session_status,
                &session_text[..session_text.len().min(200)]
            )));
        }

        let session_data: serde_json::Value = serde_json::from_str(&session_text).map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to parse session response: {}", e))
        })?;

        let session_id = session_data
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        if session_id.is_empty() {
            return Err(AutomaticallyError::ManoP(
                "No session_id returned from Mano cloud".to_string(),
            ));
        }

        log::info!(
            "[ManoPModelManager:cloud_inference] Session created: {}",
            session_id
        );

        let tool_results = vec![serde_json::json!({
            "type": "screenshot",
            "data": screenshot_b64,
        })];

        let step_body = serde_json::json!({
            "request_id": uuid::Uuid::new_v4().to_string(),
            "tool_results": tool_results,
        });

        let step_resp = client
            .post(format!(
                "{}/v1/sessions/{}/step",
                self.cloud_api_url, session_id
            ))
            .header("User-Agent", &user_agent)
            .json(&step_body)
            .timeout(std::time::Duration::from_secs(600))
            .send()
            .await
            .map_err(|e| {
                AutomaticallyError::ManoP(format!("Mano cloud step request failed: {}", e))
            })?;

        let step_status = step_resp.status();
        let step_text = step_resp.text().await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to read step response: {}", e))
        })?;

        if !step_status.is_success() {
            let _ = self.close_session(&client, &session_id, &user_agent).await;
            return Err(AutomaticallyError::ManoP(format!(
                "Mano cloud step error (HTTP {}): {}",
                step_status,
                &step_text[..step_text.len().min(200)]
            )));
        }

        let step_data: ManoStepResponse = serde_json::from_str(&step_text).map_err(|e| {
            log::warn!(
                "[ManoPModelManager:cloud_inference] Failed to parse step response: {} | body: {}",
                e,
                &step_text[..step_text.len().min(500)]
            );
            AutomaticallyError::ManoP(format!("Failed to parse step response: {}", e))
        })?;

        let status = step_data.status.as_deref().unwrap_or("RUNNING");
        let action_desc = step_data.action_desc.as_deref().unwrap_or("");
        let reasoning = step_data.reasoning.as_deref().unwrap_or("");

        let first_action = step_data
            .actions
            .first()
            .cloned()
            .unwrap_or(serde_json::json!({}));
        let action_type = first_action
            .get("type")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown")
            .to_string();

        let is_done = status == "DONE" || status == "FAIL" || status == "STOP";

        if is_done {
            let _ = self.close_session(&client, &session_id, &user_agent).await;
        }

        log::info!(
            "[ManoPModelManager:cloud_inference] Step result: status={} action_type={} action_desc='{}' reasoning='{}'",
            status,
            action_type,
            action_desc,
            reasoning
        );

        Ok(ManoCloudResponse {
            success: status != "FAIL",
            action: action_desc.to_string(),
            action_type,
            parameters: first_action,
            confidence: if status == "DONE" { 1.0 } else { 0.8 },
            model_used: "mano-cloud".to_string(),
            error: if status == "FAIL" {
                Some("Server marked task as failed".to_string())
            } else {
                None
            },
        })
    }

    /// 关闭云端会话 — 通知服务端释放会话资源
    async fn close_session(
        &self,
        client: &reqwest::Client,
        session_id: &str,
        user_agent: &str,
    ) -> Result<()> {
        let _ = client
            .post(format!(
                "{}/v1/sessions/{}/close",
                self.cloud_api_url, session_id
            ))
            .header("User-Agent", user_agent)
            .json(&serde_json::json!({"skip_eval": false}))
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await;
        log::info!(
            "[ManoPModelManager:close_session] Session {} closed",
            session_id
        );
        Ok(())
    }

    /// 获取模型存储路径 — 基于版本ID构建目录路径
    pub async fn get_model_path(&self, version: ManoPModelVersion) -> Result<PathBuf> {
        let version_dir = self.model_dir.join(version.model_id().replace("/", "-"));
        Ok(version_dir)
    }

    /// 检查模型文件是否完整 — 验证所有必需文件存在且非空
    pub async fn is_model_complete(&self, version: ManoPModelVersion) -> bool {
        let model_path = match self.get_model_path(version).await {
            Ok(p) => p,
            Err(_) => return false,
        };

        let required_files = match version {
            ManoPModelVersion::Full72B => MODEL_72B_FILES,
            ManoPModelVersion::Quantized4B => MODEL_4B_FILES,
        };

        for file in required_files {
            let file_path = model_path.join(file);
            if !file_path.exists() {
                return false;
            }
            match fs::metadata(&file_path).await {
                Ok(meta) if meta.len() > 0 => continue,
                _ => return false,
            }
        }

        log::info!(
            "[ManoPModelManager] Model {} is complete",
            version.display_name()
        );
        true
    }

    /// 下载模型 — 按优先级尝试所有下载源，逐文件下载
    pub async fn download_model(&self, version: ManoPModelVersion) -> Result<()> {
        log::warn!(
            "[ManoPModelManager:download_model] ⚠️ Mano-P model weights are NOT yet publicly available. \
             Phase 2 of the open-source roadmap is coming soon. \
             Use cloud inference (mano.mininglamp.com) instead. \
             See: https://github.com/Mininglamp-AI/Mano-P/issues/2"
        );

        let metadata = ModelMetadata::for_version(version);
        let model_path = self.get_model_path(version).await?;

        fs::create_dir_all(&model_path).await.map_err(|e| {
            AutomaticallyError::ManoP(format!(
                "Failed to create model directory {:?}: {}",
                model_path, e
            ))
        })?;

        let required_files = match version {
            ManoPModelVersion::Full72B => MODEL_72B_FILES,
            ManoPModelVersion::Quantized4B => MODEL_4B_FILES,
        };

        for source in &self.sources {
            log::info!(
                "[ManoPModelManager] Trying source: {} ({})",
                source.name,
                source.base_url
            );

            let mut all_success = true;
            for file in required_files {
                let url = metadata
                    .download_url_template
                    .replace("{base}", source.base_url)
                    .replace("{model_id}", version.model_id())
                    .replace("{filename}", file);

                let dest_path = model_path.join(file);

                if dest_path.exists() {
                    match fs::metadata(&dest_path).await {
                        Ok(meta) if meta.len() > 0 => continue,
                        _ => {}
                    }
                }

                match self.download_file(&url, &dest_path).await {
                    Ok(_) => {
                        log::info!("[ManoPModelManager] Downloaded: {}", file);
                    }
                    Err(e) => {
                        log::warn!(
                            "[ManoPModelManager] Failed to download {} from {}: {}",
                            file,
                            source.name,
                            e
                        );
                        all_success = false;
                        break;
                    }
                }
            }

            if all_success {
                return Ok(());
            }
        }

        Err(AutomaticallyError::ManoP(format!(
            "Failed to download model {} from all sources. \
             Mano-P model weights are not yet publicly available (Phase 2 coming soon). \
             Use cloud inference via mano.mininglamp.com instead. \
             See: https://github.com/Mininglamp-AI/Mano-P/issues/2",
            version.display_name()
        )))
    }

    /// 下载单个文件 — HTTP GET请求并写入本地路径
    async fn download_file(&self, url: &str, dest: &Path) -> Result<()> {
        log::debug!("[ManoPModelManager] Downloading {} -> {:?}", url, dest);

        let response = reqwest::get(url).await.map_err(|e| {
            AutomaticallyError::ManoP(format!("HTTP request failed for {}: {}", url, e))
        })?;

        if !response.status().is_success() {
            return Err(AutomaticallyError::ManoP(format!(
                "HTTP {} for {}",
                response.status(),
                url
            )));
        }

        let bytes = response.bytes().await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to read response body: {}", e))
        })?;

        let mut file = fs::File::create(dest).await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to create file {:?}: {}", dest, e))
        })?;

        file.write_all(&bytes).await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to write file {:?}: {}", dest, e))
        })?;

        Ok(())
    }

    /// 删除模型 — 递归删除模型目录及所有文件
    pub async fn remove_model(&self, version: ManoPModelVersion) -> Result<()> {
        let model_path = self.get_model_path(version).await?;
        if model_path.exists() {
            fs::remove_dir_all(&model_path).await.map_err(|e| {
                AutomaticallyError::ManoP(format!(
                    "Failed to remove model directory {:?}: {}",
                    model_path, e
                ))
            })?;
        }
        Ok(())
    }

    /// 获取模型总大小 — 遍历目录累加所有文件大小
    pub async fn get_model_size(&self, version: ManoPModelVersion) -> Result<u64> {
        let model_path = self.get_model_path(version).await?;
        if !model_path.exists() {
            return Ok(0);
        }

        let mut total_size = 0u64;
        let mut entries = fs::read_dir(&model_path).await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to read model directory: {}", e))
        })?;

        while let Some(entry) = entries.next_entry().await.map_err(|e| {
            AutomaticallyError::ManoP(format!("Failed to read directory entry: {}", e))
        })? {
            let meta = entry.metadata().await.map_err(|e| {
                AutomaticallyError::ManoP(format!("Failed to read file metadata: {}", e))
            })?;
            if meta.is_file() {
                total_size += meta.len();
            }
        }

        Ok(total_size)
    }

    /// 列出已安装的模型版本 — 检查每个版本是否完整
    pub async fn list_installed_versions(&self) -> Result<Vec<ManoPModelVersion>> {
        let mut versions = Vec::new();
        for version in [ManoPModelVersion::Quantized4B, ManoPModelVersion::Full72B] {
            if self.is_model_complete(version).await {
                versions.push(version);
            }
        }
        Ok(versions)
    }

    /// 检查本地模型支持情况 — 根据平台和架构判断是否支持本地推理
    pub fn check_local_model_support() -> LocalModelSupport {
        #[cfg(target_os = "macos")]
        {
            let is_apple_silicon = std::env::consts::ARCH == "aarch64";
            LocalModelSupport {
                platform: "macos".to_string(),
                supported: is_apple_silicon,
                reason: if is_apple_silicon {
                    "Apple Silicon Mac detected. Mano-P local model may be supported when weights are released.".to_string()
                } else {
                    "Intel Mac detected. Mano-P local model requires Apple M4 chip with 32GB+ RAM."
                        .to_string()
                },
                recommended_mode: if is_apple_silicon { "local" } else { "cloud" }.to_string(),
            }
        }

        #[cfg(target_os = "windows")]
        {
            LocalModelSupport {
                platform: "windows".to_string(),
                supported: false,
                reason: "Mano-P local model currently only supports Apple M4 Mac. Use cloud inference instead.".to_string(),
                recommended_mode: "cloud".to_string(),
            }
        }

        #[cfg(target_os = "linux")]
        {
            LocalModelSupport {
                platform: "linux".to_string(),
                supported: false,
                reason: "Mano-P local model currently only supports Apple M4 Mac. Use cloud inference instead.".to_string(),
                recommended_mode: "cloud".to_string(),
            }
        }

        #[cfg(not(any(target_os = "windows", target_os = "linux", target_os = "macos")))]
        {
            LocalModelSupport {
                platform: "unknown".to_string(),
                supported: false,
                reason: "Unsupported platform for Mano-P local model.".to_string(),
                recommended_mode: "cloud".to_string(),
            }
        }
    }
}

impl Default for ManoPModelManager {
    fn default() -> Self {
        Self::new()
    }
}

/// 本地模型支持检测结果 — 包含平台、是否支持、原因和建议模式
#[derive(Debug, Clone, serde::Serialize)]
pub struct LocalModelSupport {
    pub platform: String,
    pub supported: bool,
    pub reason: String,
    pub recommended_mode: String,
}
