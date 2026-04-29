// Claw Desktop - Mano-P 配置模块
// 模型配置、推理参数和自动化行为配置

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::error::{AutomaticallyError, Result};

/// Mano-P 模型配置
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManoPConfig {
    /// 模型版本
    pub model_version: String,
    /// 模型路径
    pub model_path: Option<PathBuf>,
    /// 推理设备 (cpu, cuda, metal)
    pub inference_device: String,
    /// 推理超时 (秒)
    pub inference_timeout_secs: u64,
    /// 最大操作步数
    pub max_action_steps: usize,
    /// 置信度阈值
    pub confidence_threshold: f32,
    /// 重试策略
    pub retry_policy: RetryPolicy,
    /// 是否启用视觉验证
    pub enable_visual_verification: bool,
    /// 视觉验证间隔 (步数)
    pub verification_interval: usize,
    /// 是否启用操作历史
    pub enable_history: bool,
    /// 历史最大长度
    pub max_history_length: usize,
    /// 是否自动重试失败的操作
    pub auto_retry_failed_actions: bool,
    /// 重试次数
    pub max_retries: u32,
}

/// 重试策略
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RetryPolicy {
    /// 最大重试次数
    pub max_attempts: u32,
    /// 初始重试延迟 (ms)
    pub initial_delay_ms: u64,
    /// 退避乘数
    pub backoff_multiplier: f64,
    /// 最大延迟 (ms)
    pub max_delay_ms: u64,
}

/// 重试策略默认值 — 3次重试，1秒初始延迟，1.5倍退避
impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 1.5,
            max_delay_ms: 10000,
        }
    }
}

/// Mano-P配置默认值 — 量化4B模型，CPU推理，30秒超时
impl Default for ManoPConfig {
    fn default() -> Self {
        Self {
            model_version: "quantized_4b".to_string(),
            model_path: None,
            inference_device: "cpu".to_string(),
            inference_timeout_secs: 30,
            max_action_steps: 50,
            confidence_threshold: 0.75,
            retry_policy: RetryPolicy::default(),
            enable_visual_verification: true,
            verification_interval: 5,
            enable_history: true,
            max_history_length: 100,
            auto_retry_failed_actions: true,
            max_retries: 3,
        }
    }
}

impl ManoPConfig {
    /// 从 JSON 字符串加载配置
    pub fn from_json(json_str: &str) -> Result<Self> {
        serde_json::from_str(json_str).map_err(|e| {
            AutomaticallyError::Config(format!("Failed to parse Mano-P config: {}", e))
        })
    }

    /// 保存配置到 JSON 字符串
    pub fn to_json(&self) -> Result<String> {
        serde_json::to_string_pretty(self).map_err(|e| {
            AutomaticallyError::Config(format!("Failed to serialize Mano-P config: {}", e))
        })
    }

    /// 从文件加载配置
    pub fn from_file(path: &PathBuf) -> Result<Self> {
        let content = std::fs::read_to_string(path).map_err(|e| {
            AutomaticallyError::Config(format!("Failed to read config file: {}", e))
        })?;
        Self::from_json(&content)
    }

    /// 保存配置到文件
    pub fn save_to_file(&self, path: &PathBuf) -> Result<()> {
        let content = self.to_json()?;
        std::fs::write(path, content).map_err(|e| {
            AutomaticallyError::Config(format!("Failed to write config file: {}", e))
        })
    }

    /// 获取模型完整路径
    pub fn get_model_full_path(&self) -> PathBuf {
        match &self.model_path {
            Some(path) => path.clone(),
            None => {
                let base_dir = dirs::home_dir()
                    .unwrap_or_else(|| PathBuf::from("."))
                    .join(".claw-desktop")
                    .join("models")
                    .join("mano-p");
                base_dir.join(&self.model_version)
            }
        }
    }

    /// 设置推理设备
    pub fn with_device(mut self, device: &str) -> Self {
        self.inference_device = device.to_string();
        self
    }

    /// 设置置信度阈值
    pub fn with_confidence_threshold(mut self, threshold: f32) -> Self {
        self.confidence_threshold = threshold.clamp(0.0, 1.0);
        self
    }

    /// 设置最大操作步数
    pub fn with_max_steps(mut self, steps: usize) -> Self {
        self.max_action_steps = steps;
        self
    }
}

/// 桌面自动化默认配置
pub fn desktop_automation_defaults() -> ManoPConfig {
    ManoPConfig {
        model_version: "quantized_4b".to_string(),
        model_path: None,
        inference_device: "cpu".to_string(),
        inference_timeout_secs: 30,
        max_action_steps: 50,
        confidence_threshold: 0.75,
        retry_policy: RetryPolicy {
            max_attempts: 3,
            initial_delay_ms: 1000,
            backoff_multiplier: 1.5,
            max_delay_ms: 10000,
        },
        enable_visual_verification: true,
        verification_interval: 5,
        enable_history: true,
        max_history_length: 100,
        auto_retry_failed_actions: true,
        max_retries: 3,
    }
}

/// 服务器端推理配置
pub fn server_inference_defaults() -> ManoPConfig {
    ManoPConfig {
        model_version: "full_72b".to_string(),
        model_path: None,
        inference_device: "cuda".to_string(),
        inference_timeout_secs: 60,
        max_action_steps: 100,
        confidence_threshold: 0.85,
        retry_policy: RetryPolicy {
            max_attempts: 5,
            initial_delay_ms: 500,
            backoff_multiplier: 2.0,
            max_delay_ms: 30000,
        },
        enable_visual_verification: true,
        verification_interval: 3,
        enable_history: true,
        max_history_length: 200,
        auto_retry_failed_actions: true,
        max_retries: 5,
    }
}

/// 轻量级配置（用于资源受限环境）
pub fn lightweight_defaults() -> ManoPConfig {
    ManoPConfig {
        model_version: "quantized_4b".to_string(),
        model_path: None,
        inference_device: "cpu".to_string(),
        inference_timeout_secs: 15,
        max_action_steps: 20,
        confidence_threshold: 0.6,
        retry_policy: RetryPolicy {
            max_attempts: 2,
            initial_delay_ms: 500,
            backoff_multiplier: 1.2,
            max_delay_ms: 5000,
        },
        enable_visual_verification: false,
        verification_interval: 10,
        enable_history: false,
        max_history_length: 10,
        auto_retry_failed_actions: false,
        max_retries: 1,
    }
}
