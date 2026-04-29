// Claw Desktop - 错误类型 - 统一错误类型定义
use thiserror::Error;

/// 统一错误类型
#[derive(Debug, Error)]
pub enum ClawError {
    #[error("Configuration error: {0}")]
    Config(String),            // 配置错误

    #[error("Database error: {0}")]
    Database(String),          // 数据库错误

    #[error("Tool execution error: {0}")]
    ToolExecution(String),     // 工具执行错误

    #[error("Channel error: {0}")]
    Channel(String),           // 渠道通信错误

    #[error("Not found: {0}")]
    NotFound(String),          // 资源未找到

    #[error("Unauthorized")]
    Unauthorized,              // 未授权

    #[error("Internal error: {0}")]
    Internal(String),          // 内部错误

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),  // IO错误

    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),  // 序列化错误
}

/// 类型别名，简化 Result 使用
pub type ClawResult<T> = Result<T, ClawError>;
