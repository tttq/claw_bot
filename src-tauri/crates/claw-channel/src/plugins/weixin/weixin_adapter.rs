// Claw Desktop - 微信适配器 - 微信消息收发
use super::context_token::{ContextTokenStore, MessageDeduplicator};
use super::ilink_api::{ILinkClient, ILinkConfig};
use super::markdown;
use crate::config::ChannelAccountConfig;
use crate::error::{ChannelError, ChannelResult};
use crate::traits::{ChannelPlugin, ConfigValidator, OutboundSender};
use crate::types::*;
use async_trait::async_trait;
use tokio::sync::RwLock;

pub struct WeixinPlugin {
    meta: ChannelMeta,
    capabilities: ChannelCapabilities,
    client: RwLock<Option<ILinkClient>>,
    #[allow(dead_code)]
    sync_buf: RwLock<String>,
    #[allow(dead_code)]
    dedup: MessageDeduplicator,
    running: RwLock<bool>,
}

impl WeixinPlugin {
    pub fn new() -> Self {
        let meta = ChannelMeta {
            id: ChannelId::WeChat,
            label: "WeChat (iLink)".to_string(),
            description: "WeChat personal account via iLink Bot API".to_string(),
            icon: Some("wechat".to_string()),
            version: "1.0.0".to_string(),
            docs_url: Some("https://ilinkai.weixin.qq.com".to_string()),
            config_fields: vec![
                ConfigFieldMeta {
                    key: "token".to_string(),
                    label: "Bot Token".to_string(),
                    field_type: ConfigFieldType::Password,
                    required: true,
                    sensitive: true,
                    placeholder: Some("iLink bot token".to_string()),
                    help_text: Some("Token from iLink Bot API".to_string()),
                    default_value: None,
                },
                ConfigFieldMeta {
                    key: "account_id".to_string(),
                    label: "Account ID".to_string(),
                    field_type: ConfigFieldType::Text,
                    required: true,
                    sensitive: false,
                    placeholder: Some("Your WeChat ID".to_string()),
                    help_text: Some("Your WeChat account ID (ilink_bot_id)".to_string()),
                    default_value: None,
                },
                ConfigFieldMeta {
                    key: "base_url".to_string(),
                    label: "API Base URL".to_string(),
                    field_type: ConfigFieldType::Url,
                    required: false,
                    sensitive: false,
                    placeholder: Some("https://ilinkai.weixin.qq.com".to_string()),
                    help_text: None,
                    default_value: Some(serde_json::Value::String(
                        "https://ilinkai.weixin.qq.com".to_string(),
                    )),
                },
                ConfigFieldMeta {
                    key: "dm_policy".to_string(),
                    label: "DM Policy".to_string(),
                    field_type: ConfigFieldType::Select(vec![
                        SelectOption {
                            value: "open".to_string(),
                            label: "Open".to_string(),
                        },
                        SelectOption {
                            value: "allowlist".to_string(),
                            label: "Allowlist".to_string(),
                        },
                        SelectOption {
                            value: "disabled".to_string(),
                            label: "Disabled".to_string(),
                        },
                    ]),
                    required: false,
                    sensitive: false,
                    placeholder: None,
                    help_text: Some("Direct message policy".to_string()),
                    default_value: Some(serde_json::Value::String("open".to_string())),
                },
                ConfigFieldMeta {
                    key: "group_policy".to_string(),
                    label: "Group Policy".to_string(),
                    field_type: ConfigFieldType::Select(vec![
                        SelectOption {
                            value: "open".to_string(),
                            label: "Open".to_string(),
                        },
                        SelectOption {
                            value: "allowlist".to_string(),
                            label: "Allowlist".to_string(),
                        },
                        SelectOption {
                            value: "disabled".to_string(),
                            label: "Disabled".to_string(),
                        },
                    ]),
                    required: false,
                    sensitive: false,
                    placeholder: None,
                    help_text: Some("Group message policy (default: disabled)".to_string()),
                    default_value: Some(serde_json::Value::String("disabled".to_string())),
                },
            ],
        };

        let capabilities = ChannelCapabilities {
            chat_types: vec![ChatType::Direct, ChatType::Group],
            supports_polls: false,
            supports_reactions: false,
            supports_edit: false,
            supports_unsend: false,
            supports_media: true,
            supports_threads: false,
            supports_streaming: false,
            max_message_length: Some(4000),
            supported_parse_modes: vec![ParseMode::PlainText],
        };

        Self {
            meta,
            capabilities,
            client: RwLock::new(None),
            sync_buf: RwLock::new(String::new()),
            dedup: MessageDeduplicator::new(),
            running: RwLock::new(false),
        }
    }

    fn build_ilink_config(&self, account_config: &ChannelAccountConfig) -> ILinkConfig {
        let auth = &account_config.auth_fields;
        ILinkConfig {
            token: auth.get("token").cloned().unwrap_or_default(),
            account_id: auth.get("account_id").cloned().unwrap_or_default(),
            base_url: auth
                .get("base_url")
                .cloned()
                .unwrap_or_else(|| "https://ilinkai.weixin.qq.com".to_string()),
            cdn_base_url: auth
                .get("cdn_base_url")
                .cloned()
                .unwrap_or_else(|| "https://novac2c.cdn.weixin.qq.com/c2c".to_string()),
        }
    }
}

#[async_trait]
impl ChannelPlugin for WeixinPlugin {
    fn meta(&self) -> &ChannelMeta {
        &self.meta
    }
    fn capabilities(&self) -> &ChannelCapabilities {
        &self.capabilities
    }

    async fn initialize(&self, account_config: &ChannelAccountConfig) -> ChannelResult<()> {
        let ilink_config = self.build_ilink_config(account_config);
        if ilink_config.token.is_empty() || ilink_config.account_id.is_empty() {
            return Err(ChannelError::Config(
                "WeChat token and account_id are required".to_string(),
            ));
        }
        let client = ILinkClient::new(ilink_config);
        *self.client.write().await = Some(client);
        log::info!(
            "[WeChat] Plugin initialized for account {}",
            account_config.id
        );
        Ok(())
    }

    async fn start(&self, account_id: &str) -> ChannelResult<()> {
        *self.running.write().await = true;
        log::info!("[WeChat] Started polling for account {}", account_id);
        Ok(())
    }

    async fn stop(&self, account_id: &str) -> ChannelResult<()> {
        *self.running.write().await = false;
        *self.client.write().await = None;
        log::info!("[WeChat] Stopped for account {}", account_id);
        Ok(())
    }

    async fn status(&self, account_id: &str) -> ChannelResult<ChannelStatus> {
        let running = *self.running.read().await;
        Ok(ChannelStatus {
            account_id: account_id.to_string(),
            channel_id: ChannelId::WeChat,
            connected: running && self.client.read().await.is_some(),
            enabled: true,
            last_activity_at: None,
            last_error: None,
            pending_messages: 0,
        })
    }
}

#[async_trait]
impl OutboundSender for WeixinPlugin {
    async fn send_text(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| ChannelError::Internal("WeChat client not initialized".to_string()))?;

        let text = match &msg.content {
            MessageContent::Text { text } => text.clone(),
            _ => {
                return Err(ChannelError::Unsupported(
                    "Only text messages supported".to_string(),
                ));
            }
        };

        let formatted = markdown::normalize_markdown_for_weixin(&text);
        let compact = msg.options.parse_mode != Some(ParseMode::PlainText);
        let chunks = markdown::split_text_for_weixin(&formatted, 4000, compact);

        let context_token = ContextTokenStore::get(&msg.account_id, &msg.target_id).await;

        for (i, chunk) in chunks.iter().enumerate() {
            if i > 0 {
                tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            }
            client
                .send_message(&msg.target_id, chunk, context_token.as_deref())
                .await
                .map_err(|e| ChannelError::Connection(e))?;
        }

        Ok(SendResult::ok(
            uuid::Uuid::new_v4().to_string(),
            ChannelId::WeChat,
        ))
    }

    async fn send_media(&self, _msg: &OutboundMessage) -> ChannelResult<SendResult> {
        Err(ChannelError::Unsupported(
            "Media sending not yet implemented for WeChat".to_string(),
        ))
    }

    async fn stream_text(
        &self,
        msg: &OutboundMessage,
        _on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<SendResult> {
        self.send_text(msg).await
    }
}

#[async_trait]
impl ConfigValidator for WeixinPlugin {
    async fn validate_config(&self, config: &serde_json::Value) -> ChannelResult<()> {
        let token = config.get("token").and_then(|v| v.as_str()).unwrap_or("");
        let account_id = config
            .get("account_id")
            .and_then(|v| v.as_str())
            .unwrap_or("");
        if token.is_empty() {
            return Err(ChannelError::Config("WeChat token is required".to_string()));
        }
        if account_id.is_empty() {
            return Err(ChannelError::Config(
                "WeChat account_id is required".to_string(),
            ));
        }
        Ok(())
    }

    fn config_schema(&self) -> Vec<ConfigFieldMeta> {
        self.meta.config_fields.clone()
    }
}
