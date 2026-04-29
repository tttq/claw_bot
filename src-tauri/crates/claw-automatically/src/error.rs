// Claw Desktop - 自动化错误 - 错误类型定义
use thiserror::Error;

/// 自动化模块错误类型 — 涵盖截图、输入、模型、推理等各类错误
#[derive(Debug, Error)]
pub enum AutomaticallyError {
    #[error("Capture error: {0}")]
    Capture(String),

    #[error("Input simulation error: {0}")]
    Input(String),

    #[error("File processing error: {0}")]
    FileProcessing(String),

    #[error("Automation error: {0}")]
    Automation(String),

    #[error("Unsupported operation: {0}")]
    UnsupportedOperation(String),

    #[error("Timeout error: {0}")]
    Timeout(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Invalid coordinates: x={0}, y={1}, screen_width={2}, screen_height={3}")]
    InvalidCoordinates(f64, f64, u32, u32),

    #[error("Mano-P model error: {0}")]
    ManoP(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Inference engine error: {0}")]
    InferenceEngine(String),

    #[error("Model not found: {0}")]
    ModelNotFound(String),

    #[error("Platform not supported: {0}")]
    PlatformNotSupported(String),
}

/// 自动化模块统一Result类型别名
pub type Result<T> = std::result::Result<T, AutomaticallyError>;
