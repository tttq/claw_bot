// Claw Desktop - WS应用状态 - WS服务器运行时状态
use claw_config::config::AppConfig;
use claw_config::path_resolver;
use claw_harness::harness::error_learning::ErrorLearningEngine;
use claw_harness::harness::observability::ObservabilityEngine;
use claw_harness::harness::persona::PersonaManager;
use std::sync::Arc;
use tokio::sync::Mutex as TokioMutex;

/// WS服务器运行时状态 — 持有所有Harness引擎和配置的共享引用
#[derive(Clone)]
pub struct AppState {
    pub config: Arc<TokioMutex<AppConfig>>,
    pub observability: Arc<ObservabilityEngine>,
    pub error_engine: Arc<TokioMutex<ErrorLearningEngine>>,
    pub persona_manager: Arc<TokioMutex<PersonaManager>>,
}

impl AppState {
    /// 创建新的AppState — 初始化所有Harness引擎
    pub fn new(config: AppConfig) -> Self {
        let agents_dir = path_resolver::get_app_root().join("agents");
        Self {
            config: Arc::new(TokioMutex::new(config)),
            observability: Arc::new(ObservabilityEngine::new()),
            error_engine: Arc::new(TokioMutex::new(ErrorLearningEngine::new())),
            persona_manager: Arc::new(TokioMutex::new(PersonaManager::new(&agents_dir))),
        }
    }

    pub async fn get_config(&self) -> AppConfig {
        self.config.lock().await.clone()
    }

    pub async fn set_config(&self, config: AppConfig) {
        *self.config.lock().await = config;
    }
}
