// Claw Desktop - 渠道配置 - 渠道账号配置管理
use sea_orm::{ActiveModelTrait, DatabaseConnection, EntityTrait, QueryOrder, Set, ModelTrait, QueryFilter, ColumnTrait};
use serde::{Deserialize, Serialize};
use crate::error::{ChannelError, ChannelResult};
use crate::types::*;
use claw_db::db::entities::channel_accounts;
use chrono;

// ====== 渠道账户配置（运行时使用）======

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChannelAccountConfig {
    pub id: String,
    pub channel_id: ChannelId,
    pub name: String,
    pub enabled: bool,

    // 认证信息（从 config_json 解析）
    pub auth_fields: std::collections::HashMap<String, String>,

    // 安全策略
    pub dm_policy: DmPolicyConfig,
    pub group_policy: GroupPolicyConfig,

    // 流式传输配置
    pub streaming_config: StreamingConfig,

    // 运行时状态
    pub status: ChannelAccountStatus,
    pub last_error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ChannelAccountStatus {
    Disabled,
    Configured,
    Connecting,
    Connected,
    Error,
    Disconnected,
}

impl ChannelAccountStatus {
    pub fn as_str(&self) -> &str {
        match self {
            ChannelAccountStatus::Disabled => "disabled",
            ChannelAccountStatus::Configured => "configured",
            ChannelAccountStatus::Connecting => "connecting",
            ChannelAccountStatus::Connected => "connected",
            ChannelAccountStatus::Error => "error",
            ChannelAccountStatus::Disconnected => "disconnected",
        }
    }

    pub fn from_str(s: &str) -> Self {
        match s {
            "disabled" => ChannelAccountStatus::Disabled,
            "configured" => ChannelAccountStatus::Configured,
            "connecting" => ChannelAccountStatus::Connecting,
            "connected" => ChannelAccountStatus::Connected,
            "error" => ChannelAccountStatus::Error,
            _ => ChannelAccountStatus::Disconnected,
        }
    }
}

// ====== DM 策略 ======

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct DmPolicyConfig {
    pub allow_from: AllowFromType,
    pub require_mention: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type")]
pub enum AllowFromType {
    Everyone,
    AllowList { users: Vec<String> },
    OwnersOnly,
}

impl Default for AllowFromType {
    fn default() -> Self {
        AllowFromType::Everyone
    }
}

// ====== 群组策略 ======

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct GroupPolicyConfig {
    pub allowed_groups: Vec<String>,
    pub require_mention: bool,
}

// ====== 配置管理器（数据库操作）======

#[derive(Clone)]
pub struct ChannelConfigManager {
    db: DatabaseConnection,
}

impl ChannelConfigManager {
    pub fn new(db: DatabaseConnection) -> Self {
        Self { db }
    }

    // 创建新账户
    pub async fn create_account(&self, account: &ChannelAccountConfig) -> ChannelResult<String> {
        let config_json = self.serialize_auth_fields(&account.auth_fields)?;

        let encrypted_fields = if !account.auth_fields.is_empty() {
            Some("bot_token,app_token,api_key,webhook_secret".to_string())
        } else {
            None
        };

        let dm_policy_str = Some(serde_json::to_string(&account.dm_policy)?);
        let group_policy_str = Some(serde_json::to_string(&account.group_policy)?);
        let streaming_str = Some(serde_json::to_string(&account.streaming_config)?);

        let now = chrono::Utc::now().naive_utc();

        let new_account = channel_accounts::ActiveModel {
            id: Set(account.id.clone()),
            channel_type: Set(account.channel_id.to_string()),
            name: Set(account.name.clone()),
            enabled: Set(account.enabled),
            config_json: Set(config_json),
            encrypted_fields: Set(encrypted_fields),
            dm_policy: Set(dm_policy_str),
            group_policy: Set(group_policy_str),
            streaming_config: Set(streaming_str),
            status: Set(account.status.as_str().to_string()),
            last_error: Set(account.last_error.clone()),
            last_connected_at: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };

        new_account.insert(&self.db).await?;

        Ok(account.id.clone())
    }

    // 获取所有账户列表
    pub async fn list_accounts(&self) -> ChannelResult<Vec<ChannelAccountConfig>> {
        let accounts = channel_accounts::Entity::find()
            .order_by_asc(channel_accounts::Column::Name)
            .all(&self.db)
            .await?;

        let mut result = Vec::new();
        for acc in accounts {
            match self.model_to_config(acc) {
                Ok(config) => result.push(config),
                Err(e) => log::warn!("Failed to parse account config: {}", e),
            }
        }

        Ok(result)
    }

    // 按渠道类型获取账户
    pub async fn list_accounts_by_channel(&self, channel_id: &ChannelId) -> ChannelResult<Vec<ChannelAccountConfig>> {
        let accounts = channel_accounts::Entity::find()
            .filter(channel_accounts::Column::ChannelType.eq(channel_id.to_string()))
            .order_by_asc(channel_accounts::Column::Name)
            .all(&self.db)
            .await?;

        let mut result = Vec::new();
        for acc in accounts {
            match self.model_to_config(acc) {
                Ok(config) => result.push(config),
                Err(e) => log::warn!("Failed to parse account config: {}", e),
            }
        }

        Ok(result)
    }

    // 获取单个账户
    pub async fn get_account(&self, account_id: &str) -> ChannelResult<ChannelAccountConfig> {
        let account = channel_accounts::Entity::find_by_id(account_id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| ChannelError::AccountNotFound(account_id.to_string()))?;

        self.model_to_config(account)
    }

    // 更新账户
    pub async fn update_account(&self, account: &ChannelAccountConfig) -> ChannelResult<()> {
        let existing = channel_accounts::Entity::find_by_id(account.id.clone())
            .one(&self.db)
            .await?
            .ok_or_else(|| ChannelError::AccountNotFound(account.id.clone()))?;

        let config_json = self.serialize_auth_fields(&account.auth_fields)?;
        let dm_policy_str = Some(serde_json::to_string(&account.dm_policy)?);
        let group_policy_str = Some(serde_json::to_string(&account.group_policy)?);
        let streaming_str = Some(serde_json::to_string(&account.streaming_config)?);

        let mut active: channel_accounts::ActiveModel = existing.into();
        active.name = Set(account.name.clone());
        active.enabled = Set(account.enabled);
        active.config_json = Set(config_json);
        active.dm_policy = Set(dm_policy_str);
        active.group_policy = Set(group_policy_str);
        active.streaming_config = Set(streaming_str);
        active.status = Set(account.status.as_str().to_string());
        active.last_error = Set(account.last_error.clone());
        active.updated_at = Set(chrono::Utc::now().naive_utc());

        active.update(&self.db).await?;

        Ok(())
    }

    // 删除账户
    pub async fn delete_account(&self, account_id: &str) -> ChannelResult<()> {
        let account = channel_accounts::Entity::find_by_id(account_id.to_string())
            .one(&self.db)
            .await?
            .ok_or_else(|| ChannelError::AccountNotFound(account_id.to_string()))?;

        account.delete(&self.db).await?;
        Ok(())
    }

    // 启用/禁用账户
    pub async fn toggle_account(&self, account_id: &str, enabled: bool) -> ChannelResult<()> {
        let account = self.get_account(account_id).await?;
        let mut updated = account;
        updated.enabled = enabled;
        updated.status = if enabled {
            ChannelAccountStatus::Configured
        } else {
            ChannelAccountStatus::Disabled
        };
        self.update_account(&updated).await
    }

    // 更新连接状态
    pub async fn update_status(
        &self,
        account_id: &str,
        status: ChannelAccountStatus,
        error: Option<String>,
    ) -> ChannelResult<()> {
        let mut account = self.get_account(account_id).await?;
        account.status = status;
        account.last_error = error;
        self.update_account(&account).await
    }

    // 辅助方法：将 Model 转换为 Config
    fn model_to_config(
        &self,
        model: channel_accounts::Model,
    ) -> ChannelResult<ChannelAccountConfig> {
        let auth_fields = self.deserialize_auth_fields(&model.config_json)?;

        let dm_policy: DmPolicyConfig = model
            .dm_policy
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let group_policy: GroupPolicyConfig = model
            .group_policy
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        let streaming_config: StreamingConfig = model
            .streaming_config
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();

        Ok(ChannelAccountConfig {
            id: model.id,
            channel_id: ChannelId::from_str(&model.channel_type),
            name: model.name,
            enabled: model.enabled,
            auth_fields,
            dm_policy,
            group_policy,
            streaming_config,
            status: ChannelAccountStatus::from_str(&model.status),
            last_error: model.last_error,
        })
    }

    fn serialize_auth_fields(&self, fields: &std::collections::HashMap<String, String>) -> ChannelResult<serde_json::Value> {
        let encrypted = self.encrypt_sensitive_fields(fields)?;
        let json = serde_json::to_value(&encrypted)?;
        Ok(json)
    }

    fn deserialize_auth_fields(&self, value: &serde_json::Value) -> ChannelResult<std::collections::HashMap<String, String>> {
        let map: std::collections::HashMap<String, String> =
            serde_json::from_value(value.clone())?;
        let decrypted = self.decrypt_sensitive_fields(&map)?;
        Ok(decrypted)
    }

    fn get_or_create_encryption_key(&self) -> ChannelResult<[u8; 32]> {
        if let Ok(k) = std::env::var("CLAW_ENCRYPTION_KEY") {
            let bytes = k.as_bytes();
            if bytes.len() >= 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&bytes[..32]);
                return Ok(key);
            }
        }

        let key_path = std::env::var("CLAW_DATA_DIR")
            .map(|d| std::path::PathBuf::from(d))
            .unwrap_or_else(|_| {
                dirs::home_dir()
                    .unwrap_or_else(|| std::path::PathBuf::from("."))
                    .join(".claw-desktop")
            })
            .join("keys")
            .join("channel_encryption.key");

        if key_path.exists() {
            let data = std::fs::read(&key_path)
                .map_err(|e| ChannelError::Internal(format!("Failed to read encryption key: {}", e)))?;
            if data.len() >= 32 {
                let mut key = [0u8; 32];
                key.copy_from_slice(&data[..32]);
                return Ok(key);
            }
        }

        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key)
            .map_err(|e| ChannelError::Internal(format!("Failed to generate encryption key: {}", e)))?;

        if let Some(parent) = key_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        std::fs::write(&key_path, &key)
            .map_err(|e| ChannelError::Internal(format!("Failed to write encryption key: {}", e)))?;

        log::info!("[ChannelConfig] Generated new channel encryption key at {:?}", key_path);
        Ok(key)
    }

    fn encrypt_sensitive_fields(&self, fields: &std::collections::HashMap<String, String>) -> ChannelResult<std::collections::HashMap<String, String>> {
        use crate::encryption::{EncryptionService, SENSITIVE_KEYS};
        
        let master_key = self.get_or_create_encryption_key()?;
        
        let service = EncryptionService::new(&master_key);
        service.encrypt_config_fields(fields, &SENSITIVE_KEYS)
            .map_err(|e| ChannelError::Internal(format!("Failed to encrypt auth fields: {}", e)))
    }

    fn decrypt_sensitive_fields(&self, fields: &std::collections::HashMap<String, String>) -> ChannelResult<std::collections::HashMap<String, String>> {
        use crate::encryption::{EncryptionService, SENSITIVE_KEYS};
        
        let master_key = self.get_or_create_encryption_key()?;
        
        let service = EncryptionService::new(&master_key);
        service.decrypt_config_fields(fields, &SENSITIVE_KEYS)
            .map_err(|e| ChannelError::Internal(format!("Failed to decrypt auth fields: {}", e)))
    }
}
