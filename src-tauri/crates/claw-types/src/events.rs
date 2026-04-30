// Claw Desktop - 事件类型 - WebSocket事件和流式事件的类型定义
use serde::{Deserialize, Serialize};

/// 应用事件枚举（用于 EventBus 跨层通信）
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum AppEvent {
    // ===== 工具事件 =====
    #[serde(rename = "tool.executed")]
    ToolExecuted {
        name: String,
        success: bool,
        duration_ms: u64,
        error: Option<String>,
    },
    #[serde(rename = "tool.registered")]
    ToolRegistered { name: String, source: String },

    // ===== Channel 事件 =====
    #[serde(rename = "channel.message_received")]
    MessageReceived {
        channel_id: String,
        account_id: String,
        chat_type: String,
        from_id: String,
        content: String,
    },
    #[serde(rename = "channel.message_sent")]
    MessageSent {
        channel_id: String,
        target_id: String,
        success: bool,
    },

    // ===== 通知事件 =====
    #[serde(rename = "notification")]
    Notification {
        level: NotificationLevel,
        title: String,
        message: String,
        #[serde(skip)]
        channels: Vec<String>,
    },

    // ===== 配置事件 =====
    #[serde(rename = "config.changed")]
    ConfigChanged {
        key: String,
        old_value: serde_json::Value,
        new_value: serde_json::Value,
    },

    // ===== 扩展事件 =====
    #[serde(rename = "extension.loaded")]
    ExtensionLoaded {
        name: String,
        version: Option<String>,
    },
    #[serde(rename = "extension.unloaded")]
    ExtensionUnloaded { name: String },

    // ===== 自动化事件 =====
    #[serde(rename = "automation.completed")]
    AutomationCompleted { instruction: String, result: String },

    // ===== 自定义事件（供插件使用）=====
    #[serde(rename = "custom")]
    Custom {
        event_type: String,
        source: String,
        payload: serde_json::Value,
    },
}

/// 通知级别
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum NotificationLevel {
    Info,
    Warning,
    Error,
    Success,
}
