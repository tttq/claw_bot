// Claw Desktop - 凭证池 - 管理多API Key的轮询和负载均衡
// 当某个API Key被限流时自动切换到下一个可用Key，实现多Key轮询和限流恢复

use claw_db::db::get_db;
use sea_orm::{ConnectionTrait, Statement};
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicUsize, Ordering};
use tokio::sync::RwLock;

/// 凭证条目 - 表示一个API Key及其状态信息
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CredentialEntry {
    pub id: String,                        // 凭证唯一标识
    pub provider: String,                  // 服务商名称（如 openai, anthropic）
    pub api_key: String,                   // API密钥
    pub base_url: Option<String>,          // 自定义API基础URL
    pub is_active: bool,                   // 是否激活可用
    pub rate_limit_remaining: Option<i64>, // 剩余限流配额
    pub rate_limit_reset_at: Option<i64>,  // 限流重置时间（Unix时间戳）
    pub last_used_at: Option<i64>,         // 上次使用时间（Unix时间戳）
}

/// 凭证池 - 管理多个API Key，支持轮询选择和限流感知
pub struct CredentialPool {
    entries: RwLock<Vec<CredentialEntry>>, // 凭证条目列表（读写锁保护）
    current_index: AtomicUsize,            // 当前轮询索引（原子操作）
}

impl CredentialPool {
    /// 创建新的空凭证池
    pub fn new() -> Self {
        Self {
            entries: RwLock::new(Vec::new()),
            current_index: AtomicUsize::new(0),
        }
    }

    /// 从数据库加载凭证列表
    pub async fn load_from_db(&self) -> Result<(), String> {
        let db = get_db().await;
        let rows = db.query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT id, provider, api_key, base_url, is_active, rate_limit_remaining, rate_limit_reset_at, last_used_at FROM credential_pool",
            [],
        )).await.map_err(|e| e.to_string())?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(CredentialEntry {
                id: row.try_get::<String>("", "id").unwrap_or_default(),
                provider: row.try_get::<String>("", "provider").unwrap_or_default(),
                api_key: row.try_get::<String>("", "api_key").unwrap_or_default(),
                base_url: row.try_get::<Option<String>>("", "base_url").ok().flatten(),
                is_active: row.try_get::<bool>("", "is_active").unwrap_or(false),
                rate_limit_remaining: row
                    .try_get::<Option<i64>>("", "rate_limit_remaining")
                    .ok()
                    .flatten(),
                rate_limit_reset_at: row
                    .try_get::<Option<i64>>("", "rate_limit_reset_at")
                    .ok()
                    .flatten(),
                last_used_at: row
                    .try_get::<Option<i64>>("", "last_used_at")
                    .ok()
                    .flatten(),
            });
        }

        let mut guard = self.entries.write().await;
        *guard = entries;
        log::info!("[CredentialPool] Loaded {} credentials", guard.len());
        Ok(())
    }

    /// 获取下一个可用的API Key（轮询策略，跳过被限流的Key）
    pub async fn get_next_key(&self) -> Option<CredentialEntry> {
        let entries = self.entries.read().await;
        if entries.is_empty() {
            return None;
        }
        let now = chrono::Utc::now().timestamp();
        let len = entries.len();
        for _ in 0..len {
            let idx = self.current_index.fetch_add(1, Ordering::Relaxed) % len;
            let entry = &entries[idx];
            if !entry.is_active {
                continue;
            }
            if let Some(reset_at) = entry.rate_limit_reset_at {
                if now < reset_at && entry.rate_limit_remaining.unwrap_or(0) <= 0 {
                    continue;
                }
            }
            return Some(entry.clone());
        }
        entries.iter().find(|e| e.is_active).cloned()
    }

    /// 标记指定凭证被限流，记录限流重置时间
    pub async fn mark_rate_limited(&self, credential_id: &str, reset_at: Option<i64>) {
        let mut entries = self.entries.write().await;
        if let Some(entry) = entries.iter_mut().find(|e| e.id == credential_id) {
            entry.rate_limit_remaining = Some(0);
            entry.rate_limit_reset_at = reset_at;
            log::warn!(
                "[CredentialPool] Key {} rate limited",
                claw_types::truncate_str_safe(&credential_id, 8)
            );
        }
    }

    /// 添加新凭证到池中，同时持久化到数据库
    pub async fn add_credential(&self, entry: CredentialEntry) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR REPLACE INTO credential_pool (id, provider, api_key, base_url, is_active, rate_limit_remaining, rate_limit_reset_at, last_used_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            [
                entry.id.clone().into(), entry.provider.clone().into(), entry.api_key.clone().into(),
                entry.base_url.clone().into(), entry.is_active.into(),
                entry.rate_limit_remaining.into(), entry.rate_limit_reset_at.into(),
                entry.last_used_at.into(), now.into(),
            ],
        )).await.map_err(|e| e.to_string())?;
        let mut entries = self.entries.write().await;
        entries.push(entry);
        Ok(())
    }

    /// 从池中移除指定凭证，同时从数据库删除
    pub async fn remove_credential(&self, id: &str) -> Result<(), String> {
        let db = get_db().await;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM credential_pool WHERE id = ?1",
            [id.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
        let mut entries = self.entries.write().await;
        entries.retain(|e| e.id != id);
        Ok(())
    }

    /// 列出所有凭证条目
    pub async fn list_credentials(&self) -> Vec<CredentialEntry> {
        self.entries.read().await.clone()
    }
}
