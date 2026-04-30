// Claw Desktop - LLM引擎 - Agent循环引擎（消息发送→工具调用→结果聚合）
use claw_config::config::AppConfig;
use std::sync::Arc;

pub struct LlmEngine {
    pub config: Arc<std::sync::Mutex<AppConfig>>,
}

impl LlmEngine {
    pub fn new(config: AppConfig) -> Self {
        Self {
            config: Arc::new(std::sync::Mutex::new(config)),
        }
    }

    pub fn get_config(&self) -> AppConfig {
        self.config.lock().map(|g| g.clone()).unwrap_or_default()
    }
}
