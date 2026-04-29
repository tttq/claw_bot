// Claw Desktop - 渠道库 - 外部消息渠道（Discord/Telegram/微信）集成
pub mod error;
pub mod types;
pub mod traits;
pub mod config;
pub mod registry;
pub mod streaming;
pub mod plugins;
pub mod encryption;
pub mod inbound;
pub mod bootstrap;

pub use error::{ChannelError, ChannelResult};
pub use types::*;
pub use traits::*;
pub use config::*;
pub use registry::ChannelRegistry;
pub use encryption::{EncryptionService, SENSITIVE_KEYS, get_sensitive_keys_for_channel};
pub use inbound::{InboundPipeline, ProcessedMessage};
pub use bootstrap::*;
