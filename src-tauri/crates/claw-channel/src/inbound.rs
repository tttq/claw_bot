// Claw Desktop - 入站消息 - 处理从渠道接收的消息
use crate::error::{ChannelError, ChannelResult};
use crate::types::*;
use std::collections::HashSet;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

pub struct InboundPipeline {
    debounce: DebouncePolicy,
    mention_gating: MentionGating,
}

impl InboundPipeline {
    pub fn new() -> Self {
        Self {
            debounce: DebouncePolicy::new(),
            mention_gating: MentionGating::new(),
        }
    }

    pub async fn process(
        &self,
        message: InboundMessage,
        config: &crate::config::DmPolicyConfig,
        group_config: &crate::config::GroupPolicyConfig,
    ) -> ChannelResult<ProcessedMessage> {
        // 1. 检查是否为重复消息
        if self.debounce.is_duplicate(&message).await {
            return Err(ChannelError::Internal("Duplicate message".to_string()));
        }

        // 2. DM 策略检查
        if message.chat_type == ChatType::Direct {
            self.check_dm_policy(&message, config)?;
        }

        // 3. 群组策略检查
        if message.chat_type == ChatType::Group {
            self.check_group_policy(&message, group_config)?;

            // 4. @提及检测
            if !self
                .mention_gating
                .should_process(&message, group_config.require_mention)
            {
                return Err(ChannelError::Internal(
                    "Mention required but not found".to_string(),
                ));
            }
        }

        Ok(ProcessedMessage {
            original: message,
            processed_at: chrono::Utc::now(),
            metadata: std::collections::HashMap::new(),
        })
    }

    fn check_dm_policy(
        &self,
        message: &InboundMessage,
        policy: &crate::config::DmPolicyConfig,
    ) -> ChannelResult<()> {
        match &policy.allow_from {
            crate::config::AllowFromType::Everyone => Ok(()),
            crate::config::AllowFromType::AllowList { users } => {
                if users.contains(&message.sender_id) {
                    Ok(())
                } else {
                    Err(ChannelError::Internal(format!(
                        "User {} not in allow list",
                        message.sender_id
                    )))
                }
            }
            crate::config::AllowFromType::OwnersOnly => {
                Err(ChannelError::Internal("DM only for owners".to_string()))
            }
        }
    }

    fn check_group_policy(
        &self,
        message: &InboundMessage,
        policy: &crate::config::GroupPolicyConfig,
    ) -> ChannelResult<()> {
        if policy.allowed_groups.is_empty() || policy.allowed_groups.contains(&message.chat_id) {
            Ok(())
        } else {
            Err(ChannelError::Internal(format!(
                "Group {} not in allowed list",
                message.chat_id
            )))
        }
    }
}

impl Default for InboundPipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone)]
pub struct ProcessedMessage {
    pub original: InboundMessage,
    pub processed_at: chrono::DateTime<chrono::Utc>,
    pub metadata: std::collections::HashMap<String, serde_json::Value>,
}

// ====== 防抖/去重策略 ======

pub struct DebouncePolicy {
    seen_messages: RwLock<HashSet<String>>,
    seen_at: RwLock<std::collections::HashMap<String, Instant>>,
    window_ms: u64,
}

impl DebouncePolicy {
    pub fn new() -> Self {
        Self {
            seen_messages: RwLock::new(HashSet::new()),
            seen_at: RwLock::new(std::collections::HashMap::new()),
            window_ms: 5000,
        }
    }

    pub async fn is_duplicate(&self, message: &InboundMessage) -> bool {
        let key = format!(
            "{}:{}:{}",
            message.channel_id,
            message.message_id,
            message.content.to_display_string()
        );

        let mut seen = self.seen_messages.write().await;
        if seen.contains(&key) {
            true
        } else {
            seen.insert(key.clone());
            drop(seen);

            let mut seen_at = self.seen_at.write().await;
            seen_at.insert(key, Instant::now());
            false
        }
    }

    pub async fn cleanup(&self) {
        let mut seen = self.seen_messages.write().await;
        let mut seen_at = self.seen_at.write().await;

        let now = Instant::now();
        let expired: Vec<String> = seen_at
            .iter()
            .filter(|&(_, &t)| now.duration_since(t) > Duration::from_millis(self.window_ms))
            .map(|(k, _)| k.clone())
            .collect();

        for key in expired {
            seen.remove(&key);
            seen_at.remove(&key);
        }
    }
}

// ====== @提及检测 ======

pub struct MentionGating {
    bot_usernames: RwLock<Vec<String>>,
}

impl MentionGating {
    pub fn new() -> Self {
        Self {
            bot_usernames: RwLock::new(Vec::new()),
        }
    }

    pub async fn set_bot_usernames(&self, usernames: Vec<String>) {
        *self.bot_usernames.write().await = usernames;
    }

    pub fn should_process(&self, message: &InboundMessage, require_mention: bool) -> bool {
        if !require_mention {
            return true;
        }

        let text = match &message.content {
            MessageContent::Text { text } => text,
            _ => return false,
        };

        let bot_usernames = self.bot_usernames.blocking_read();

        if bot_usernames.is_empty() {
            return true;
        }

        bot_usernames.iter().any(|username| {
            text.contains(&format!("@{}", username))
                || text
                    .to_lowercase()
                    .contains(&format!("@{}", username.to_lowercase()))
        })
    }
}
