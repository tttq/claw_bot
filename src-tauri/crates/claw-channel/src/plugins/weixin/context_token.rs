// Claw Desktop - 上下文令牌 - 微信上下文Token管理
use claw_db::db::get_db;
use sea_orm::{ConnectionTrait, Statement};
use std::time::Duration;

pub struct ContextTokenStore;

impl ContextTokenStore {
    pub async fn get(account_id: &str, user_id: &str) -> Option<String> {
        let db = get_db().await;
        let rows = db.query_all(Statement::from_sql_and_values(
            db.get_database_backend(),
            "SELECT context_token FROM weixin_context_tokens WHERE account_id = ?1 AND user_id = ?2",
            [account_id.into(), user_id.into()],
        )).await.ok()?;

        rows.first()
            .and_then(|row| row.try_get::<String>("", "context_token").ok())
    }

    pub async fn set(account_id: &str, user_id: &str, token: &str) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR REPLACE INTO weixin_context_tokens (account_id, user_id, context_token, updated_at) VALUES (?1, ?2, ?3, ?4)",
            [account_id.into(), user_id.into(), token.into(), now.into()],
        )).await.map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct MessageDeduplicator {
    ttl: Duration,
}

impl MessageDeduplicator {
    pub fn new() -> Self {
        Self {
            ttl: Duration::from_secs(300),
        }
    }

    pub async fn is_duplicate(&self, message_id: &str) -> bool {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - self.ttl.as_secs() as i64;

        if let Ok(rows) = db
            .query_all(Statement::from_sql_and_values(
                db.get_database_backend(),
                "SELECT 1 FROM message_dedup WHERE message_id = ?1 AND seen_at > ?2",
                [message_id.into(), cutoff.into()],
            ))
            .await
        {
            !rows.is_empty()
        } else {
            false
        }
    }

    pub async fn mark_seen(&self, message_id: &str) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "INSERT OR REPLACE INTO message_dedup (message_id, seen_at) VALUES (?1, ?2)",
            [message_id.into(), now.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;

        let _ = self.cleanup_expired().await;
        Ok(())
    }

    async fn cleanup_expired(&self) -> Result<(), String> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let cutoff = now - self.ttl.as_secs() as i64;
        db.execute(Statement::from_sql_and_values(
            db.get_database_backend(),
            "DELETE FROM message_dedup WHERE seen_at < ?1",
            [cutoff.into()],
        ))
        .await
        .map_err(|e| e.to_string())?;
        Ok(())
    }
}
