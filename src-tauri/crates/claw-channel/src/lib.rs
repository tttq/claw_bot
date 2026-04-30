// Claw Desktop - 渠道库 - 外部消息渠道（Discord/Telegram/微信）集成
pub mod bootstrap;
pub mod config;
pub mod encryption;
pub mod error;
pub mod inbound;
pub mod plugins;
pub mod registry;
pub mod streaming;
pub mod traits;
pub mod types;

pub use bootstrap::*;
pub use config::*;
pub use encryption::{EncryptionService, SENSITIVE_KEYS, get_sensitive_keys_for_channel};
pub use error::{ChannelError, ChannelResult};
pub use inbound::{InboundPipeline, ProcessedMessage};
pub use registry::ChannelRegistry;
pub use traits::*;
pub use types::*;
