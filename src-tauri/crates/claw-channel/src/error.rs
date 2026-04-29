// Claw Desktop - 渠道错误 - 渠道相关错误类型
use thiserror::Error;

#[derive(Debug, Error)]
pub enum ChannelError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Connection error: {0}")]
    Connection(String),

    #[error("Authentication failed: {0}")]
    Auth(String),

    #[error("Rate limited, retry after {0:?} seconds")]
    RateLimited(Option<u64>),

    #[error("Message too long: {0} chars exceeds limit of {1}")]
    MessageTooLong(usize, usize),

    #[error("Unsupported operation for this channel: {0}")]
    Unsupported(String),

    #[error("Plugin not found: {0}")]
    PluginNotFound(String),

    #[error("Account not found: {0}")]
    AccountNotFound(String),

    #[error("Database error: {0}")]
    Database(String),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Internal error: {0}")]
    Internal(String),
}

impl From<serde_json::Error> for ChannelError {
    fn from(err: serde_json::Error) -> Self {
        ChannelError::Serialization(err.to_string())
    }
}

impl From<sea_orm::DbErr> for ChannelError {
    fn from(err: sea_orm::DbErr) -> Self {
        ChannelError::Database(err.to_string())
    }
}

pub type ChannelResult<T> = Result<T, ChannelError>;
