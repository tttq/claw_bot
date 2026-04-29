// Claw Desktop - 渠道类型 - 渠道消息和账号的类型定义
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ====== 渠道标识符 ======

#[derive(Debug, Clone, Serialize, Deserialize, Hash, Eq, PartialEq)]
pub enum ChannelId {
    Telegram,
    Discord,
    Slack,
    WhatsApp,
    Signal,
    WeChat,
    WeCom,
    Feishu,
    DingTalk,
    Custom(String),
}

impl ChannelId {
    pub fn as_str(&self) -> &str {
        match self {
            ChannelId::Telegram => "telegram",
            ChannelId::Discord => "discord",
            ChannelId::Slack => "slack",
            ChannelId::WhatsApp => "whatsapp",
            ChannelId::Signal => "signal",
            ChannelId::WeChat => "weixin",
            ChannelId::WeCom => "wecom",
            ChannelId::Feishu => "feishu",
            ChannelId::DingTalk => "dingtalk",
            ChannelId::Custom(name) => name.as_str(),
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "telegram" => ChannelId::Telegram,
            "discord" => ChannelId::Discord,
            "slack" => ChannelId::Slack,
            "whatsapp" => ChannelId::WhatsApp,
            "signal" => ChannelId::Signal,
            "weixin" | "wechat" => ChannelId::WeChat,
            "wecom" => ChannelId::WeCom,
            "feishu" | "lark" => ChannelId::Feishu,
            "dingtalk" => ChannelId::DingTalk,
            other => ChannelId::Custom(other.to_string()),
        }
    }
}

impl std::fmt::Display for ChannelId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ====== 聊天类型 ======

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChatType {
    Direct,
    Group,
    Channel,
    Thread,
}

impl ChatType {
    pub fn as_str(&self) -> &str {
        match self {
            ChatType::Direct => "direct",
            ChatType::Group => "group",
            ChatType::Channel => "channel",
            ChatType::Thread => "thread",
        }
    }
}

// ====== 渠道元数据 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelMeta {
    pub id: ChannelId,
    pub label: String,
    pub description: String,
    pub icon: Option<String>,
    pub version: String,
    pub docs_url: Option<String>,
    pub config_fields: Vec<ConfigFieldMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConfigFieldMeta {
    pub key: String,
    pub label: String,
    pub field_type: ConfigFieldType,
    pub required: bool,
    pub sensitive: bool,       // 是否是敏感信息（如 Token）
    pub placeholder: Option<String>,
    pub help_text: Option<String>,
    pub default_value: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ConfigFieldType {
    Text,
    Password,
    Number,
    Boolean,
    Select(Vec<SelectOption>),
    TextArea,
    Url,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SelectOption {
    pub value: String,
    pub label: String,
}

// ====== 渠道能力声明 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelCapabilities {
    pub chat_types: Vec<ChatType>,
    pub supports_polls: bool,
    pub supports_reactions: bool,
    pub supports_edit: bool,
    pub supports_unsend: bool,
    pub supports_media: bool,
    pub supports_threads: bool,
    pub supports_streaming: bool,
    pub max_message_length: Option<usize>,
    pub supported_parse_modes: Vec<ParseMode>,
}

impl Default for ChannelCapabilities {
    fn default() -> Self {
        Self {
            chat_types: vec![ChatType::Direct],
            supports_polls: false,
            supports_reactions: false,
            supports_edit: false,
            supports_unsend: false,
            supports_media: false,
            supports_threads: false,
            supports_streaming: false,
            max_message_length: None,
            supported_parse_modes: vec![ParseMode::PlainText],
        }
    }
}

// ====== 解析模式 ======

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ParseMode {
    Markdown,
    Html,
    PlainText,
}

impl ParseMode {
    pub fn as_str(&self) -> &str {
        match self {
            ParseMode::Markdown => "markdown",
            ParseMode::Html => "html",
            ParseMode::PlainText => "plain",
        }
    }
}

// ====== 消息内容类型 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MessageContent {
    Text { text: String },
    Media {
        url: String,
        mime_type: String,
        caption: Option<String>,
    },
    Poll {
        question: String,
        options: Vec<String>,
    },
}

impl MessageContent {
    pub fn text_content(&self) -> Option<&str> {
        match self {
            MessageContent::Text { text } => Some(text),
            _ => None,
        }
    }

    pub fn to_display_string(&self) -> String {
        match self {
            MessageContent::Text { text } => text.clone(),
            MessageContent::Media { caption, .. } => {
                caption.clone().unwrap_or_else(|| "[Media]".to_string())
            }
            MessageContent::Poll { question, .. } => format!("[Poll] {}", question),
        }
    }
}

// ====== 入站消息 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InboundMessage {
    pub message_id: String,
    pub channel_id: ChannelId,
    pub account_id: String,
    pub sender_id: String,
    pub sender_name: Option<String>,
    pub sender_username: Option<String>,
    pub chat_id: String,
    pub chat_type: ChatType,
    pub content: MessageContent,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub reply_to_message_id: Option<String>,
    pub thread_id: Option<String>,
    pub metadata: HashMap<String, serde_json::Value>,
}

impl InboundMessage {
    pub fn new(
        message_id: String,
        channel_id: ChannelId,
        account_id: String,
        sender_id: String,
        chat_id: String,
        chat_type: ChatType,
        content: MessageContent,
    ) -> Self {
        Self {
            message_id,
            channel_id,
            account_id,
            sender_id,
            sender_name: None,
            sender_username: None,
            chat_id,
            chat_type,
            content,
            timestamp: chrono::Utc::now(),
            reply_to_message_id: None,
            thread_id: None,
            metadata: HashMap::new(),
        }
    }
}

// ====== 出站消息 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutboundMessage {
    pub channel_id: ChannelId,
    pub account_id: String,
    pub target_id: String,
    pub target_chat_type: ChatType,
    pub content: MessageContent,
    pub reply_to_message_id: Option<String>,
    pub thread_id: Option<String>,
    pub options: OutboundOptions,
}

impl OutboundMessage {
    pub fn new(
        channel_id: ChannelId,
        account_id: String,
        target_id: String,
        target_chat_type: ChatType,
        content: MessageContent,
    ) -> Self {
        Self {
            channel_id,
            account_id,
            target_id,
            target_chat_type,
            content,
            reply_to_message_id: None,
            thread_id: None,
            options: OutboundOptions::default(),
        }
    }

    pub fn with_reply(mut self, message_id: String) -> Self {
        self.reply_to_message_id = Some(message_id);
        self
    }

    pub fn with_thread(mut self, thread_id: String) -> Self {
        self.thread_id = Some(thread_id);
        self
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct OutboundOptions {
    pub silent: bool,
    pub parse_mode: Option<ParseMode>,
    pub preview_url: bool,
}

// ====== 发送结果 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendResult {
    pub success: bool,
    pub message_id: Option<String>,
    pub error: Option<String>,
    pub channel_id: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

impl SendResult {
    pub fn ok(message_id: String, channel_id: ChannelId) -> Self {
        Self {
            success: true,
            message_id: Some(message_id),
            error: None,
            channel_id: channel_id.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }

    pub fn err(error: impl Into<String>, channel_id: ChannelId) -> Self {
        Self {
            success: false,
            message_id: None,
            error: Some(error.into()),
            channel_id: channel_id.to_string(),
            timestamp: chrono::Utc::now(),
        }
    }
}

// ====== 渠道状态 ======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelStatus {
    pub account_id: String,
    pub channel_id: ChannelId,
    pub connected: bool,
    pub enabled: bool,
    pub last_activity_at: Option<chrono::DateTime<chrono::Utc>>,
    pub last_error: Option<String>,
    pub pending_messages: u32,
}

// ====== 流式传输配置 ======

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum StreamingMode {
    Off,
    Partial,
    Block,
}

impl StreamingMode {
    pub fn as_str(&self) -> &str {
        match self {
            StreamingMode::Off => "off",
            StreamingMode::Partial => "partial",
            StreamingMode::Block => "block",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StreamingConfig {
    pub enabled: bool,
    pub mode: StreamingMode,
    pub chunk_size: Option<usize>,
    pub edit_delay_ms: Option<u64>,  // 编辑消息延迟（用于模拟流式）
}

impl Default for StreamingConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            mode: StreamingMode::Partial,
            chunk_size: None,
            edit_delay_ms: Some(500),
        }
    }
}
