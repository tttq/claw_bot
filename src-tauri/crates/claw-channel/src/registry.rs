// Claw Desktop - 渠道注册表 - 管理所有渠道适配器
use crate::config::{ChannelAccountConfig, ChannelConfigManager};
use crate::error::{ChannelError, ChannelResult};
use crate::traits::{ChannelPlugin, OutboundSender};
use crate::types::*;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

pub struct ChannelRegistry {
    plugins: RwLock<HashMap<String, Arc<dyn ChannelPlugin>>>,
    senders: RwLock<HashMap<String, Arc<dyn OutboundSender>>>,
    config_manager: Option<ChannelConfigManager>,
}

impl ChannelRegistry {
    pub fn new() -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            senders: RwLock::new(HashMap::new()),
            config_manager: None,
        }
    }

    pub fn with_db(db: sea_orm::DatabaseConnection) -> Self {
        Self {
            plugins: RwLock::new(HashMap::new()),
            senders: RwLock::new(HashMap::new()),
            config_manager: Some(ChannelConfigManager::new(db)),
        }
    }

    // 注册插件
    pub async fn register<P>(&self, plugin: P)
    where
        P: ChannelPlugin + OutboundSender + 'static,
    {
        let channel_id = plugin.meta().id.to_string();
        let plugin_arc = Arc::new(plugin);
        let sender_arc = plugin_arc.clone() as Arc<dyn OutboundSender>;

        let mut plugins = self.plugins.write().await;
        plugins.insert(channel_id.clone(), plugin_arc);

        let mut senders = self.senders.write().await;
        senders.insert(channel_id.clone(), sender_arc);

        log::info!("[ChannelRegistry] Registered plugin: {}", channel_id);
    }

    // 获取已注册的渠道列表
    pub async fn list_registered_channels(&self) -> Vec<ChannelMeta> {
        let plugins = self.plugins.read().await;
        plugins.values().map(|p| p.meta().clone()).collect()
    }

    // 初始化所有账户（从数据库加载配置）
    pub async fn initialize_all(&self) -> ChannelResult<()> {
        if let Some(ref manager) = self.config_manager {
            let accounts = manager.list_accounts().await?;

            for account in accounts {
                if !account.enabled {
                    continue;
                }

                match self.initialize_account(&account).await {
                    Ok(_) => {
                        log::info!(
                            "[ChannelRegistry] Initialized account: {} ({})",
                            account.name,
                            account.id
                        );
                    }
                    Err(e) => {
                        log::error!(
                            "[ChannelRegistry] Failed to initialize account {}: {}",
                            account.id,
                            e
                        );
                        if let Some(ref mgr) = self.config_manager {
                            let _ = mgr
                                .update_status(
                                    &account.id,
                                    crate::config::ChannelAccountStatus::Error,
                                    Some(e.to_string()),
                                )
                                .await;
                        }
                    }
                }
            }
        }

        Ok(())
    }

    // 初始化单个账户
    pub async fn initialize_account(&self, account: &ChannelAccountConfig) -> ChannelResult<()> {
        let channel_id = account.channel_id.to_string();
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(&channel_id)
            .ok_or_else(|| ChannelError::PluginNotFound(channel_id.clone()))?;

        plugin.initialize(account).await?;

        if let Some(ref manager) = self.config_manager {
            manager
                .update_status(
                    &account.id,
                    crate::config::ChannelAccountStatus::Configured,
                    None,
                )
                .await?;
        }

        Ok(())
    }

    // 启动所有账户
    pub async fn start_all(&self) -> ChannelResult<()> {
        if let Some(ref manager) = self.config_manager {
            let accounts = manager.list_accounts().await?;

            for account in accounts {
                if !account.enabled {
                    continue;
                }

                if let Err(e) = self.start_account(&account.id).await {
                    log::error!("[ChannelRegistry] Failed to start {}: {}", account.id, e);
                }
            }
        }

        Ok(())
    }

    // 启动单个账户
    pub async fn start_account(&self, account_id: &str) -> ChannelResult<()> {
        let account = match &self.config_manager {
            Some(manager) => manager.get_account(account_id).await?,
            None => return Err(ChannelError::Internal("No database connection".to_string())),
        };

        let channel_id = account.channel_id.to_string();
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(&channel_id)
            .ok_or_else(|| ChannelError::PluginNotFound(channel_id.clone()))?;

        plugin.start(account_id).await?;

        if let Some(ref manager) = self.config_manager {
            manager
                .update_status(
                    account_id,
                    crate::config::ChannelAccountStatus::Connected,
                    None,
                )
                .await?;
        }

        Ok(())
    }

    // 停止账户
    pub async fn stop_account(&self, account_id: &str) -> ChannelResult<()> {
        let account = match &self.config_manager {
            Some(manager) => manager.get_account(account_id).await?,
            None => return Err(ChannelError::Internal("No database connection".to_string())),
        };

        let channel_id = account.channel_id.to_string();
        let plugins = self.plugins.read().await;

        if let Some(plugin) = plugins.get(&channel_id) {
            plugin.stop(account_id).await?;
        }

        if let Some(ref manager) = self.config_manager {
            manager
                .update_status(
                    account_id,
                    crate::config::ChannelAccountStatus::Disconnected,
                    None,
                )
                .await?;
        }

        Ok(())
    }

    // 发送消息（核心方法）
    pub async fn send_message(&self, msg: &OutboundMessage) -> ChannelResult<SendResult> {
        let channel_id = msg.channel_id.to_string();
        let senders = self.senders.read().await;

        let sender = senders
            .get(&channel_id)
            .ok_or_else(|| ChannelError::PluginNotFound(channel_id.clone()))?;

        match &msg.content {
            MessageContent::Text { .. } => sender.send_text(msg).await,
            MessageContent::Media { .. } => sender.send_media(msg).await,
            MessageContent::Poll { .. } => Err(ChannelError::Unsupported(
                "Poll sending not implemented yet".to_string(),
            )),
        }
    }

    // 流式发送消息
    pub async fn stream_message(
        &self,
        msg: &OutboundMessage,
        on_token: Box<dyn Fn(String) + Send + Sync>,
    ) -> ChannelResult<SendResult> {
        let channel_id = msg.channel_id.to_string();
        let senders = self.senders.read().await;

        let sender = senders
            .get(&channel_id)
            .ok_or_else(|| ChannelError::PluginNotFound(channel_id.clone()))?;

        sender.stream_text(msg, on_token).await
    }

    // 获取所有账户状态
    pub async fn get_all_statuses(&self) -> ChannelResult<Vec<ChannelStatus>> {
        match &self.config_manager {
            Some(manager) => {
                let accounts = manager.list_accounts().await?;
                let mut statuses = Vec::new();

                for account in accounts {
                    let status = self.get_account_status(&account.id).await?;
                    statuses.push(status);
                }

                Ok(statuses)
            }
            None => Ok(Vec::new()),
        }
    }

    // 获取单个账户状态
    pub async fn get_account_status(&self, account_id: &str) -> ChannelResult<ChannelStatus> {
        let account = match &self.config_manager {
            Some(manager) => manager.get_account(account_id).await?,
            None => return Err(ChannelError::Internal("No database connection".to_string())),
        };

        let channel_id = account.channel_id.to_string();
        let plugins = self.plugins.read().await;

        let (connected, last_activity, error) = if let Some(plugin) = plugins.get(&channel_id) {
            match plugin.status(account_id).await {
                Ok(status) => (status.connected, status.last_activity_at, status.last_error),
                Err(e) => (false, None, Some(e.to_string())),
            }
        } else {
            (false, None, None)
        };

        Ok(ChannelStatus {
            account_id: account.id,
            channel_id: account.channel_id,
            connected,
            enabled: account.enabled,
            last_activity_at: last_activity,
            last_error: error.or(account.last_error),
            pending_messages: 0,
        })
    }

    // 获取配置管理器引用
    pub fn config_manager(&self) -> Option<&ChannelConfigManager> {
        self.config_manager.as_ref()
    }

    // 测试连接
    pub async fn test_connection(&self, account_id: &str) -> ChannelResult<bool> {
        let account = match &self.config_manager {
            Some(manager) => manager.get_account(account_id).await?,
            None => return Err(ChannelError::Internal("No database connection".to_string())),
        };

        let channel_id = account.channel_id.to_string();
        let plugins = self.plugins.read().await;

        let plugin = plugins
            .get(&channel_id)
            .ok_or_else(|| ChannelError::PluginNotFound(channel_id.clone()))?;

        plugin.initialize(&account).await?;
        plugin.start(account_id).await?;

        let status = plugin.status(account_id).await?;
        plugin.stop(account_id).await?;

        Ok(status.connected)
    }
}

impl Default for ChannelRegistry {
    fn default() -> Self {
        Self::new()
    }
}
