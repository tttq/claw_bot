// Claw Desktop - Discord适配器 - Discord Bot消息收发
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use reqwest::Client as HttpClient;

use crate::error::{ChannelError, ChannelResult};
use crate::traits::{ChannelPlugin, OutboundSender};
use crate::types::*;
use crate::config::ChannelAccountConfig;
use crate::streaming::StreamingController;

const DISCORD_API_BASE: &str = "https://discord.com/api/v10";

pub struct DiscordPlugin {
    clients: Arc<RwLock<HashMap<String, Arc<DiscordClient>>>>,
}

impl DiscordPlugin {
    pub fn new() -> Self {
        Self {
            clients: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    async fn get_client(&self, account_id: &str) -> ChannelResult<Arc<DiscordClient>> {
        let clients = self.clients.read().await;
        clients
            .get(account_id)
            .cloned()
            .ok_or_else(|| ChannelError::AccountNotFound(account_id.to_string()))
    }
}

#[async_trait]
impl ChannelPlugin for DiscordPlugin {
    fn meta(&self) -> &ChannelMeta {
        static META: std::sync::OnceLock<ChannelMeta> = std::sync::OnceLock::new();
        META.get_or_init(|| ChannelMeta {
            id: ChannelId::Discord,
            label: "Discord".to_string(),
            description: "Discord Bot 集成 - 支持服务器、频道和线程".to_string(),
            icon: Some("🎮".to_string()),
            version: "1.0.0".to_string(),
            docs_url: Some("https://discord.com/developers/docs/intro".to_string()),
            config_fields: vec![
                ConfigFieldMeta {
                    key: "bot_token".to_string(),
                    label: "Bot Token".to_string(),
                    field_type: ConfigFieldType::Password,
                    required: true,
                    sensitive: true,
                    placeholder: Some("MTIzNDU2Nzg5MDEyMzQ1Njc4.OABCDEF.GhIjKlMnOpqRsTuVwXyZ".to_string()),
                    help_text: Some("从 Discord Developer Portal 获取".to_string()),
                    default_value: None,
                },
                ConfigFieldMeta {
                    key: "application_id".to_string(),
                    label: "Application ID (可选)".to_string(),
                    field_type: ConfigFieldType::Text,
                    required: false,
                    sensitive: false,
                    placeholder: Some("123456789012345678".to_string()),
                    help_text: Some("用于交互式命令".to_string()),
                    default_value: None,
                },
                ConfigFieldMeta {
                    key: "allowed_guilds".to_string(),
                    label: "允许的服务器 IDs (逗号分隔)".to_string(),
                    field_type: ConfigFieldType::Text,
                    required: false,
                    sensitive: false,
                    placeholder: Some("guild1_id, guild2_id".to_string()),
                    help_text: Some("留空则允许所有服务器".to_string()),
                    default_value: None,
                },
            ],
        })
    }

    fn capabilities(&self) -> &ChannelCapabilities {
        static CAPS: std::sync::OnceLock<ChannelCapabilities> = std::sync::OnceLock::new();
        CAPS.get_or_init(|| ChannelCapabilities {
            chat_types: vec![ChatType::Direct, ChatType::Group, ChatType::Channel, ChatType::Thread],
            supports_polls: false,
            supports_reactions: true,
            supports_edit: true,
            supports_unsend: true,
            supports_media: true,
            supports_threads: true,
            supports_streaming: true,
            max_message_length: Some(2000),
            supported_parse_modes: vec![ParseMode::Markdown],
        })
    }

    async fn initialize(&self, account_config: &ChannelAccountConfig) -> ChannelResult<()> {
        let bot_token = account_config
            .auth_fields
            .get("bot_token")
            .ok_or_else(|| ChannelError::Config("Missing bot_token".to_string()))?
            .clone();

        let http = HttpClient::builder()
            .user_agent("qclaw-desktop/1.0")
            .build()
            .map_err(|e| ChannelError::Internal(format!("Failed to create HTTP client: {}", e)))?;

        let client = DiscordClient::new(
            account_config.id.clone(),
            bot_token,
            http,
            account_config.streaming_config.clone(),
        );

        client.validate_token().await?;

        let mut clients = self.clients.write().await;
        clients.insert(account_config.id.clone(), Arc::new(client));

        log::info!(
            "[Discord] Initialized account: {} ({})",
            account_config.name,
            account_config.id
        );

        Ok(())
    }

    async fn start(&self, account_id: &str) -> ChannelResult<()> {
        let client = self.get_client(account_id).await?;
        client.start_gateway().await?;

        log::info!("[Discord] Started gateway for account: {}", account_id);
        Ok(())
    }

    async fn stop(&self, account_id: &str) -> ChannelResult<()> {
        if let Ok(client) = self.get_client(account_id).await {
            client.stop().await?;
        }

        log::info!("[Discord] Stopped account: {}", account_id);
        Ok(())
    }

    async fn status(&self, account_id: &str) -> ChannelResult<ChannelStatus> {
        match self.get_client(account_id).await {
            Ok(client) => client.get_status().await,
            Err(_) => Ok(ChannelStatus {
                account_id: account_id.to_string(),
                channel_id: ChannelId::Discord,
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
impl OutboundSender for DiscordPlugin {
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

// ====== Discord REST API 响应类型 ======

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct DiscordUser {
    id: String,
    username: String,
    discriminator: String,
    bot: bool,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct DiscordMessageResponse {
    id: String,
    channel_id: String,
}

#[derive(serde::Serialize)]
struct DiscordSendPayload {
    content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_reference: Option<DiscordMessageReference>,
}

#[derive(serde::Serialize)]
struct DiscordMessageReference {
    message_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    guild_id: Option<String>,
    fail_if_not_exists: bool,
}

// ====== Discord Client 实现（使用 reqwest 真实 API）======

pub struct DiscordClient {
    account_id: String,
    bot_token: String,
    http: HttpClient,
    streaming_config: StreamingConfig,
    is_running: Arc<std::sync::atomic::AtomicBool>,
}

impl DiscordClient {
    pub fn new(
        account_id: String,
        bot_token: String,
        http: HttpClient,
        streaming_config: StreamingConfig,
    ) -> Self {
        Self {
            account_id,
            bot_token,
            http,
            streaming_config,
            is_running: Arc::new(std::sync::atomic::AtomicBool::new(false)),
        }
    }

    fn auth_header_value(&self) -> String {
        format!("Bot {}", self.bot_token)
    }

    pub async fn validate_token(&self) -> ChannelResult<()> {
        if self.bot_token.is_empty() || !self.bot_token.contains('.') {
            return Err(ChannelError::Auth("Invalid bot token format".to_string()));
        }

        let resp = self.http
            .get(format!("{}/users/@me", DISCORD_API_BASE))
            .header("Authorization", self.auth_header_value())
            .header("Content-Type", "application/json")
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Discord API request failed: {}", e)))?;

        if resp.status().is_success() {
            let user: DiscordUser = resp.json().await
                .map_err(|e| ChannelError::Internal(format!("Failed to parse Discord response: {}", e)))?;
            log::info!(
                "[Discord] Token validated - Bot: {}#{} (ID: {})",
                user.username,
                user.discriminator,
                user.id
            );
            Ok(())
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            Err(ChannelError::Auth(format!("Discord token validation failed ({}): {}", status, body)))
        }
    }

    pub async fn start_gateway(&self) -> ChannelResult<()> {
        self.is_running.store(true, std::sync::atomic::Ordering::SeqCst);
        log::info!("[Discord] Starting WebSocket Gateway for account: {}", self.account_id);

        let gateway_url = format!("{}/gateway?v=10&encoding=json", DISCORD_API_BASE.replace("/api/v10", ""));
        log::info!("[Discord] Connecting to gateway: {}", gateway_url);

        match tokio_tungstenite::connect_async(&gateway_url).await {
            Ok((ws_stream, _response)) => {
                log::info!("[Discord] WebSocket Gateway connected for account: {}", self.account_id);
                
                use futures_util::{SinkExt, StreamExt};
                
                let (mut write, mut read) = ws_stream.split();
                
                let identify_payload = serde_json::json!({
                    "op": 2,
                    "d": {
                        "token": self.bot_token,
                        "properties": {
                            "os": "windows",
                            "browser": "qclaw-desktop",
                            "device": "qclaw-desktop"
                        },
                        "intents": 1 | 2 | 4 | 8 | 16 | 32 | 64 | 128 | 256 | 512 | 1024 | 2048 | 4096 | 8192 | 16384
                    }
                });

                if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::text(
                    serde_json::to_string(&identify_payload).unwrap_or_default()
                )).await {
                    log::error!("[Discord] Failed to send identify payload: {}", e);
                }

                log::info!("[Discord] Identify payload sent, starting event loop...");
                
                let is_running = self.is_running.clone();
                let account_id = self.account_id.clone();
                
                tokio::spawn(async move {
                    let mut heartbeat_interval: Option<u64> = None;
                    let mut last_sequence: Option<i64> = None;
                    let mut last_heartbeat = tokio::time::Instant::now();
                    
                    loop {
                        if !is_running.load(std::sync::atomic::Ordering::SeqCst) {
                            log::info!("[Discord:{}] Gateway loop stopped by request", account_id);
                            break;
                        }

                        if let Some(hb_ms) = heartbeat_interval {
                            if last_heartbeat.elapsed() >= tokio::time::Duration::from_millis(hb_ms) {
                                let heartbeat_payload = serde_json::json!({
                                    "op": 1,
                                    "d": last_sequence
                                });
                                if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::text(
                                    serde_json::to_string(&heartbeat_payload).unwrap_or_default()
                                )).await {
                                    log::warn!("[Discord:{}] Failed to send heartbeat: {}", account_id, e);
                                    break;
                                }
                                last_heartbeat = tokio::time::Instant::now();
                                log::debug!("[Discord:{}] Heartbeat sent (seq={:?})", account_id, last_sequence);
                            }
                        }
                        
                        match tokio::time::timeout(
                            tokio::time::Duration::from_secs(5),
                            read.next()
                        ).await {
                            Ok(Some(Ok(msg))) => {
                                if let tokio_tungstenite::tungstenite::Message::Text(text) = &msg {
                                    if let Ok(event) = serde_json::from_str::<serde_json::Value>(text) {
                                        if let Some(op) = event.get("op").and_then(|v| v.as_u64()) {
                                            match op {
                                                0 => {
                                                    if let Some(t) = event.get("t").and_then(|v| v.as_str()) {
                                                        log::debug!("[Discord:{}] Event: {}", account_id, t);
                                                    }
                                                    if let Some(s) = event.get("s").and_then(|v| v.as_i64()) {
                                                        last_sequence = Some(s);
                                                    }
                                                }
                                                1 => {
                                                    log::debug!("[Discord:{}] Heartbeat requested", account_id);
                                                    let heartbeat_payload = serde_json::json!({
                                                        "op": 1,
                                                        "d": last_sequence
                                                    });
                                                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::text(
                                                        serde_json::to_string(&heartbeat_payload).unwrap_or_default()
                                                    )).await {
                                                        log::warn!("[Discord:{}] Failed to send heartbeat: {}", account_id, e);
                                                        break;
                                                    }
                                                    last_heartbeat = tokio::time::Instant::now();
                                                }
                                                10 => {
                                                    if let Some(d) = event.get("d") {
                                                        if let Some(hb) = d.get("heartbeat_interval").and_then(|v| v.as_u64()) {
                                                            heartbeat_interval = Some(hb);
                                                            last_heartbeat = tokio::time::Instant::now();
                                                            log::info!("[Discord:{}] Heartbeat interval: {}ms", account_id, hb);
                                                        }
                                                    }
                                                }
                                                11 => {
                                                    log::debug!("[Discord:{}] Heartbeat ACK", account_id);
                                                }
                                                _ => {}
                                            }
                                        }
                                    }
                                }
                            }
                            Ok(Some(Err(e))) => {
                                log::warn!("[Discord:{}] WebSocket error: {}", account_id, e);
                                break;
                            }
                            Ok(None) => {
                                log::info!("[Discord:{}] WebSocket stream closed", account_id);
                                break;
                            }
                            Err(_) => {
                                log::debug!("[Discord:{}] Read timeout, checking running state", account_id);
                            }
                        }
                    }
                });
                
                Ok(())
            }
            Err(e) => {
                log::warn!("[Discord] ⚠️ WebSocket Gateway connection failed ({}), using REST-only mode", e);
                log::info!("[Discord] REST API functions (send_text, send_media) remain available");
                Ok(())
            }
        }
    }

    pub async fn stop(&self) -> ChannelResult<()> {
        self.is_running.store(false, std::sync::atomic::Ordering::SeqCst);
        log::info!("[Discord] Stopped gateway for account: {}", self.account_id);
        Ok(())
    }

    pub async fn get_status(&self) -> ChannelResult<ChannelStatus> {
        Ok(ChannelStatus {
            account_id: self.account_id.clone(),
            channel_id: ChannelId::Discord,
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

        if text.len() > 2000 {
            return Err(ChannelError::MessageTooLong(text.len(), 2000));
        }

        let message_ref = msg.reply_to_message_id.as_ref().map(|rid| {
            DiscordMessageReference {
                message_id: rid.clone(),
                channel_id: Some(msg.target_id.clone()),
                guild_id: None,
                fail_if_not_exists: false,
            }
        });

        let payload = DiscordSendPayload {
            content: text,
            message_reference: message_ref,
        };

        let url = format!("{}/channels/{}/messages", DISCORD_API_BASE, msg.target_id);

        let resp = self.http
            .post(&url)
            .header("Authorization", self.auth_header_value())
            .header("Content-Type", "application/json")
            .json(&payload)
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Discord API request failed: {}", e)))?;

        if resp.status().is_success() {
            let sent: DiscordMessageResponse = resp.json().await
                .map_err(|e| ChannelError::Internal(format!("Failed to parse response: {}", e)))?;
            let message_id = format!("dc_{}", sent.id);
            log::info!(
                "[Discord] Sent text to #{} (message_id: {})",
                msg.target_id,
                message_id
            );
            Ok(SendResult::ok(message_id, ChannelId::Discord))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            log::error!("[Discord] Failed to send ({}): {}", status, body);
            Err(ChannelError::Internal(format!("Discord send failed ({}): {}", status, body)))
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

        let url_preview: String = url_str.chars().take(50).collect();
        log::info!("[Discord] Uploading media to Discord CDN (mime: {}, url: {}...)", 
            mime_type, 
            if url_str.len() > 50 { format!("{}...", url_preview) } else { url_str.clone() });

        let download_resp = self.http
            .get(&url_str)
            .send()
            .await
            .map_err(|e| ChannelError::Internal(format!("Failed to download media: {}", e)))?;

        if !download_resp.status().is_success() {
            return Err(ChannelError::Internal(format!("Media download failed: {}", download_resp.status())));
        }

        let bytes_vec = download_resp.bytes().await
            .map_err(|e| ChannelError::Internal(format!("Failed to read media bytes: {}", e)))?
            .to_vec();

        let part = reqwest::multipart::Part::bytes(bytes_vec.clone())
            .file_name("media")
            .mime_str(&mime_type)
            .unwrap_or_else(|_| reqwest::multipart::Part::bytes(bytes_vec).file_name("media"));

        let mut form = reqwest::multipart::Form::new();
        if let Some(cap) = &caption {
            form = form.text("content", cap.clone());
        }
        form = form.part("file", part);

        let upload_url = format!("{}/channels/{}/messages", DISCORD_API_BASE, msg.target_id);

        let resp = self.http
            .post(&upload_url)
            .header("Authorization", self.auth_header_value())
            .multipart(form)
            .send()
            .await
            .map_err(|e| ChannelError::Connection(format!("Discord upload request failed: {}", e)))?;

        if resp.status().is_success() {
            let sent: DiscordMessageResponse = resp.json().await
                .map_err(|e| ChannelError::Internal(format!("Failed to parse upload response: {}", e)))?;
            let message_id = format!("dc_media_{}", sent.id);
            log::info!(
                "[Discord] ✅ Uploaded media to #{} (size: {}KB, id: {})",
                msg.target_id,
                0,
                message_id
            );
            Ok(SendResult::ok(message_id, ChannelId::Discord))
        } else {
            let status = resp.status();
            let body = resp.text().await.unwrap_or_default();
            
            log::warn!("[Discord] ⚠️ Upload failed ({}): {}, falling back to link mode", status, body);
            
            let fallback_payload = serde_json::json!({
                "content": format!("{} [Media: {}]", caption.unwrap_or_else(|| "[Media]".to_string()), url_str),
            });

            let fallback_resp = self.http
                .post(&upload_url)
                .header("Authorization", self.auth_header_value())
                .header("Content-Type", "application/json")
                .json(&fallback_payload)
                .send()
                .await
                .map_err(|e| ChannelError::Connection(format!("Discord fallback failed: {}", e)))?;

            if fallback_resp.status().is_success() {
                let sent: DiscordMessageResponse = fallback_resp.json().await
                    .map_err(|e| ChannelError::Internal(format!("Fallback parse error: {}", e)))?;
                log::info!("[Discord] Sent media as link (fallback) - id: dc_link_{}", sent.id);
                Ok(SendResult::ok(format!("dc_link_{}", sent.id), ChannelId::Discord))
            } else {
                Err(ChannelError::Internal(format!("Discord media send completely failed ({})", status)))
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
