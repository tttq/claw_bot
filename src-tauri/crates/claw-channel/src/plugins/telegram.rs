// Claw Desktop - Telegram适配器 - Telegram Bot消息收发
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use teloxide::prelude::*;
use teloxide::types::{ParseMode as TgParseMode, ChatId, MessageId};

use crate::error::{ChannelError, ChannelResult};
use crate::traits::{ChannelPlugin, OutboundSender};
use crate::types::*;
use crate::config::ChannelAccountConfig;
use crate::streaming::StreamingController;

pub struct TelegramPlugin {
    clients: Arc<RwLock<HashMap<String, Arc<TelegramClient>>>>,
}

impl TelegramPlugin {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_client(&self, account_id: &str) -> ChannelResult<Arc<TelegramClient>> {
        let clients = self.clients.read().await;
        clients
            .get(account_id)
            .cloned()
            .ok_or_else(|| ChannelError::AccountNotFound(account_id.to_string()))
    }
}

#[async_trait]
impl ChannelPlugin for TelegramPlugin {
    fn meta(&self) -> &ChannelMeta {
        static META: std::sync::OnceLock<ChannelMeta> = std::sync::OnceLock::new();
        META.get_or_init(|| ChannelMeta {
            id: ChannelId::Telegram,
            label: "Telegram".to_string(),
            description: "Telegram Bot 集成 - 支持私聊、群组和频道".to_string(),
            icon: Some("📱".to_string()),
            version: "1.0.0".to_string(),
            docs_url: Some("https://core.telegram.org/bots/api".to_string()),
            config_fields: vec![
                ConfigFieldMeta {
                    key: "bot_token".to_string(),
                    label: "Bot Token".to_string(),
                    field_type: ConfigFieldType::Password,
                    required: true,
                    sensitive: true,
                    placeholder: Some("123456789:ABCdefGHIjklMNOpqrsTUVwxyz".to_string()),
                    help_text: Some("从 @BotFather 获取".to_string()),
                    default_value: None,
                },
                ConfigFieldMeta {
                    key: "webhook_url".to_string(),
                    label: "Webhook URL (可选)".to_string(),
                    field_type: ConfigFieldType::Url,
                    required: false,
                    sensitive: false,
                    placeholder: Some("https://your-server.com/webhook/telegram".to_string()),
                    help_text: Some("留空则使用轮询模式 (推荐)".to_string()),
                    default_value: None,
                },
            ],
        })
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        static CAPS: std::sync::OnceLock<ChannelCapabilities> = std::sync::OnceLock::new();
        CAPS.get_or_init(|| ChannelCapabilities {
            chat_types: vec![ChatType::Direct, ChatType::Group, ChatType::Channel],
            supports_polls: true,
            supports_reactions: true,
            supports_edit: true,
            supports_unsend: true,
            supports_media: true,
            supports_threads: false,
            supports_streaming: true,
            max_message_length: Some(4096),
            supported_parse_modes: vec![ParseMode::Markdown, ParseMode::Html, ParseMode::PlainText],
        })
    }

    async fn initialize(&self, account_config: &ChannelAccountConfig) -> ChannelResult<()> {
        let bot_token = account_config
            .auth_fields
            .get("bot_token")
            .ok_or_else(|| ChannelError::Config("Missing bot_token".to_string()))?
            .clone();

        let client = TelegramClient::new(
            account_config.id.clone(),
            bot_token,
            account_config.streaming_config.clone(),
        );

        client.validate_token().await?;

        let mut clients = self.clients.write().await;
        clients.insert(account_config.id.clone(), Arc::new(client));

        log::info!(
            "[Telegram] Initialized account: {} ({})",
            account_config.name,
            account_config.id
        );

        Ok(())
    }

    async fn start(&self, account_id: &str) -> ChannelResult<()> {
        let client = self.get_client(account_id).await?;
        client.start_polling().await?;

        log::info!("[Telegram] Started polling for account: {}", account_id);
        Ok(())
    }

    async fn stop(&self, account_id: &str) -> ChannelResult<()> {
        if let Ok(client) = self.get_client(account_id).await {
            client.stop().await?;
        }

        log::info!("[Telegram] Stopped account: {}", account_id);
        Ok(())
    }

    async fn status(&self, account_id: &str) -> ChannelResult<ChannelStatus> {
        match self.get_client(account_id).await {
            Ok(client) => client.get_status().await,
            Err(_) => Ok(ChannelStatus {
                account_id: account_id.to_string(),
                channel_id: ChannelId::Telegram,
                connected: false,
                enabled: false,
                last_activity_at: None,
                last_error: Some("Not initialized".to_string()),
                pending_messages: 0,
            }),
        }
    }
}

#[async_trait]
impl OutboundSender for TelegramPlugin {
    async fn send_text(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        let client = self.get_client(&msg.account_id).await?;
        client.send_text(msg).await
    }

    async fn send_media(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        let client = self.get_client(&msg.account_id).await?;
        client.send_media(msg).await
    }

    async fn stream_text(
        &self,
        msg: &OutboundMessage,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<SendResult> {
        let client = self.get_client(&msg.account_id).await?;
        client.stream_text(msg, on_token).await
    }
}

fn convert_parse_mode(pm: Option<ParseMode>) -> Option<TgParseMode> {
    match pm {
        Some(ParseMode::Html) => Some(TgParseMode::Html),
        Some(ParseMode::Markdown) => Some(TgParseMode::MarkdownV2),
        Some(ParseMode::PlainText) => None,
        None => None,
    }
}

// ====== Telegram Client 实现（使用 teloxide 真实 API）======

pub struct TelegramClient {
    account_id: String,
    bot_token: String,
    streaming_config: StreamingConfig,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl TelegramClient {
    pub fn new(
        account_id: String,
        bot_token: String,
        streaming_config: StreamingConfig,
    ) -> Self {
        Self {
            account_id,
            bot_token,
            streaming_config,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    pub async fn validate_token(&self) -> ChannelResult<()> {
        if self.bot_token.is_empty() || !self.bot_token.contains(':') {
            return Err(ChannelError::Auth("Invalid bot token format".to_string()));
        }

        let bot = Bot::new(&self.bot_token);

        match bot.get_me().send().await {
            Ok(me) => {
                let username = me.username
                    .as_ref()
                    .map(|s| s.as_str())
                    .unwrap_or_default()
                    .to_string();
                log::info!(
                    "[Telegram] Token validated - Bot: @{} (ID: {})",
                    username,
                    me.id
                );
                Ok(())
            }
            Err(e) => Err(ChannelError::Auth(format!("Token validation failed: {}", e))),
        }
    }

    pub async fn start_polling(&self) -> ChannelResult<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("[Telegram] Starting polling for account: {}", self.account_id);

        let bot = Bot::new(self.bot_token.clone());
        
        let account_id = self.account_id.clone();
        
        let is_running = self.is_running.clone();
        
        tokio::spawn(async move {
            log::info!("[Telegram:{}] Polling loop started", account_id);
            
            let mut offset: i32 = 0;
            let mut err_count: u32 = 0;
            
            loop {
                if !is_running.load(std::sync::atomic::Ordering::SeqCst) {
                    log::info!("[Telegram:{}] Polling stopped by request", account_id);
                    break;
                }
                
                match bot.get_updates()
                    .offset(offset)
                    .limit(100)
                    .timeout(30)
                    .await
                {
                    Ok(updates) => {
                        err_count = 0;
                        for update in updates {
                            offset = update.id + 1;
                            
                            if let teloxide::types::UpdateKind::Message(msg) = update.kind {
                                let text_preview = msg.text().map(|t| {
                                    let preview: String = t.chars().take(50).collect();
                                    if t.chars().count() > 50 { format!("{}...", preview) } else { preview }
                                }).unwrap_or_default();
                                log::info!(
                                    "[Telegram:{}] Received from {} (chat:{}): {:?}...",
                                    account_id,
                                    msg.from().map(|u| u.first_name.as_str()).unwrap_or("?"),
                                    msg.chat.id,
                                    text_preview
                                );
                            }
                        }
                    }
                    Err(e) => {
                        err_count += 1;
                        if err_count > 10 {
                            log::error!("[Telegram:{}] Too many errors, stopping poll", account_id);
                            break;
                        }
                        log::warn!("[Telegram:{}] Polling error #{}: {} (retry...)", account_id, err_count, e);
                        tokio::time::sleep(tokio::time::Duration::from_secs((5 * err_count.min(5)) as u64)).await;
                    }
                }
            }
            
            log::info!("[Telegram:{}] Polling stopped", account_id);
        });

        log::info!("[Telegram] ✅ Polling started for account: {}", self.account_id);
        Ok(())
    }

    pub async fn stop(&self) -> ChannelResult<()> {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        log::info!("[Telegram] Stopped polling for account: {}", self.account_id);
        Ok(())
    }

    pub async fn get_status(&self) -> ChannelResult<ChannelStatus> {
        Ok(ChannelStatus {
            account_id: self.account_id.clone(),
            channel_id: ChannelId::Telegram,
            connected: self.is_running.load(std::sync::atomic::Ordering::SeqCst),
            enabled: true,
            last_activity_at: Some(chrono::Utc::now()),
            last_error: None,
            pending_messages: 0,
        })
    }

    pub async fn send_text(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        if !self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ChannelError::Connection("Client not running".to_string()));
        }

        let text = match &msg.content {
            MessageContent::Text { text } => text.clone(),
            _ => return Err(ChannelError::Unsupported("Expected text content".to_string())),
        };

        if text.len() > 4096 {
            return Err(ChannelError::MessageTooLong(text.len(), 4096));
        }

        let bot = Bot::new(&self.bot_token);
        let chat_id = ChatId(msg.target_id.parse::<i64>().map_err(|_| {
            ChannelError::Internal(format!("Invalid chat ID: {}", msg.target_id))
        })?);

        let tg_parse_mode = convert_parse_mode(msg.options.parse_mode.clone());

        let mut request = bot.send_message(chat_id, text);

        if let Some(ref pm) = tg_parse_mode {
            request = request.parse_mode(pm.clone());
        }

        if msg.options.silent {
            request = request.disable_notification(true);
        }

        if let Some(ref reply_id) = msg.reply_to_message_id {
            if let Ok(reply_i32) = reply_id.parse::<i32>() {
                request = request.reply_to_message_id(MessageId(reply_i32));
            }
        }

        match request.send().await {
            Ok(sent_msg) => {
                let message_id = format!("tg_{}", sent_msg.id);
                log::info!(
                    "[Telegram] Sent text to {} (message_id: {}, api_id: {})",
                    msg.target_id,
                    message_id,
                    sent_msg.id
                );
                Ok(SendResult::ok(message_id, ChannelId::Telegram))
            }
            Err(e) => {
                log::error!("[Telegram] Failed to send: {}", e);
                Err(ChannelError::Internal(format!("Send failed: {}", e)))
            }
        }
    }

    pub async fn send_media(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        if !self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ChannelError::Connection("Client not running".to_string()));
        }

        let (url_str, mime_type, caption) = match &msg.content {
            MessageContent::Media { url, mime_type, caption } => {
                (url.clone(), mime_type.clone(), caption.clone())
            }
            _ => return Err(ChannelError::Unsupported("Expected media content".to_string())),
        };

        let bot = Bot::new(&self.bot_token);
        let chat_id = ChatId(msg.target_id.parse::<i64>().map_err(|_| {
            ChannelError::Internal(format!("Invalid chat ID: {}", msg.target_id))
        })?);

        let parsed_url = url::Url::parse(&url_str).map_err(|e| {
            ChannelError::Internal(format!("Invalid media URL: {}", e))
        })?;
        let input = teloxide::types::InputFile::url(parsed_url);

        let result: Result<String, teloxide::RequestError> = match mime_type.as_str() {
            m if m.starts_with("image/") => {
                bot.send_photo(chat_id, input)
                    .caption(caption.unwrap_or_default())
                    .send()
                    .await
                    .map(|m| format!("tg_photo_{}", m.id))
            }
            m if m.starts_with("video/") => {
                bot.send_video(chat_id, input)
                    .caption(caption.unwrap_or_default())
                    .send()
                    .await
                    .map(|m| format!("tg_video_{}", m.id))
            }
            _ => {
                bot.send_document(chat_id, input)
                    .caption(caption.unwrap_or_default())
                    .send()
                    .await
                    .map(|m| format!("tg_doc_{}", m.id))
            }
        };

        match result {
            Ok(message_id) => {
                log::info!(
                    "[Telegram] Sent media to {} (mime: {}, id: {})",
                    msg.target_id,
                    mime_type,
                    message_id
                );
                Ok(SendResult::ok(message_id, ChannelId::Telegram))
            }
            Err(e) => {
                log::error!("[Telegram] Failed to send media: {}", e);
                Err(ChannelError::Internal(format!("Media send failed: {}", e)))
            }
        }
    }

    pub async fn stream_text(
        &self,
        msg: &OutboundMessage,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<SendResult> {
        if !self.is_running.load(std::sync::atomic::Ordering::SeqCst) {
            return Err(ChannelError::Connection("Client not running".to_string()));
        }

        let text = match &msg.content {
            MessageContent::Text { text } => text.clone(),
            _ => return Err(ChannelError::Unsupported("Expected text content".to_string())),
        };

        let on_token = Arc::from(on_token);
        let controller = StreamingController::new(self.streaming_config.clone());
        let final_text = controller.process_stream(text, on_token).await?;

        let final_msg = OutboundMessage {
            content: MessageContent::Text { text: final_text },
            ..msg.clone()
        };

        self.send_text(&final_msg).await
    }
}
