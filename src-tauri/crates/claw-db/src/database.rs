// Claw Desktop - 数据库核心 - Sea-ORM数据库连接池和CRUD操作（Sea-ORM 1.1 稳定版）
// conversations / messages / 增强记忆系统 v2 (memory_units, entities, memory_links)

use crate::db::entities::conversations::Entity as Conversations;
use crate::db::entities::messages::Entity as Messages;
use crate::db::{get_db, init_main_db, try_get_agent_db};
use log;
use sea_orm::prelude::*;
use sea_orm::{ActiveModelTrait, EntityTrait, QueryOrder, Set, Statement};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// 会话数据传输对象
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Conversation {
    pub id: String,      // 会话唯一ID
    pub title: String,   // 会话标题
    pub created_at: i64, // 创建时间（Unix时间戳）
    pub updated_at: i64, // 更新时间（Unix时间戳）
    #[serde(default)]
    pub message_count: u64, // 消息数量
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_id: Option<String>, // 关联的Agent ID
}

/// 消息数据传输对象
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Message {
    pub id: String,               // 消息唯一ID
    pub conversation_id: String,  // 所属会话ID
    pub role: String,             // 角色 (user/assistant/tool/system)
    pub content: String,          // 消息内容
    pub timestamp: i64,           // 时间戳
    pub token_count: Option<i32>, // Token数量
    #[serde(skip_serializing)]
    pub embedding: Option<Vec<u8>>, // 向量嵌入（不序列化到前端）
    pub is_error: bool,           // 是否为错误消息
    pub model: Option<String>,    // 使用的模型名称
    pub metadata: Option<String>, // 元数据JSON字符串
}

impl From<crate::db::entities::conversations::Model> for Conversation {
    fn from(m: crate::db::entities::conversations::Model) -> Self {
        Conversation {
            id: m.id,
            title: m.title,
            created_at: m.created_at,
            updated_at: m.updated_at,
            message_count: std::cmp::max(m.message_count, 0) as u64,
            agent_id: None,
        }
    }
}

impl From<crate::db::entities::messages::Model> for Message {
    fn from(m: crate::db::entities::messages::Model) -> Self {
        Message {
            id: m.id,
            conversation_id: m.conversation_id,
            role: m.role,
            content: m.content,
            timestamp: m.timestamp,
            token_count: m.token_count,
            embedding: m.embedding,
            is_error: m.is_error != 0,
            model: m.model,
            metadata: m.metadata,
        }
    }
}

pub struct Database;

impl Database {
    /// 初始化主数据库：创建 conversations/messages/vectors 三张表 + 索引 + WAL 模式 + 外键约束
    pub async fn init(db_path: &PathBuf) -> Result<(), anyhow::Error> {
        let db_path_str = db_path
            .to_str()
            .ok_or_else(|| anyhow::anyhow!("db path is not valid UTF-8: {:?}", db_path))?;
        let db = init_main_db(db_path_str).await?;
        log::info!("[DB] 初始化主数据库表结构...");
        let backend = db.get_database_backend();
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS conversations (
                id TEXT PRIMARY KEY,
                title TEXT NOT NULL DEFAULT 'New Conversation',
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL,
                message_count INTEGER NOT NULL DEFAULT 0,
                metadata TEXT DEFAULT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS messages (
                id TEXT PRIMARY KEY,
                conversation_id TEXT NOT NULL,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                token_count INTEGER DEFAULT NULL,
                embedding BLOB DEFAULT NULL,
                is_error INTEGER NOT NULL DEFAULT 0,
                model TEXT DEFAULT NULL,
                metadata TEXT DEFAULT NULL,
                FOREIGN KEY(conversation_id) REFERENCES conversations(id) ON DELETE CASCADE
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_messages_conv ON messages(conversation_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_messages_time ON messages(timestamp)",
        ))
        .await?;

        // ===== 增强记忆系统 v2 (Hindsight-inspired) =====
        log::info!("[DB] 初始化增强记忆系统表...");
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS memory_units (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                conversation_id TEXT,
                text TEXT NOT NULL,
                embedding BLOB NOT NULL,
                fact_type TEXT NOT NULL DEFAULT 'world',
                context TEXT DEFAULT NULL,
                occurred_at INTEGER DEFAULT NULL,
                mentioned_at INTEGER DEFAULT NULL,
                source_type TEXT NOT NULL DEFAULT 'conversation',
                metadata TEXT DEFAULT NULL,
                tags TEXT DEFAULT NULL,
                importance_score REAL NOT NULL DEFAULT 1.0,
                access_count INTEGER NOT NULL DEFAULT 0,
                memory_layer TEXT DEFAULT NULL,
                expires_at INTEGER DEFAULT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS entities (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                canonical_name TEXT NOT NULL,
                entity_type TEXT NOT NULL DEFAULT 'general',
                metadata TEXT DEFAULT NULL,
                first_seen INTEGER NOT NULL,
                last_seen INTEGER NOT NULL,
                mention_count INTEGER NOT NULL DEFAULT 1
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS unit_entities (
                unit_id TEXT NOT NULL,
                entity_id TEXT NOT NULL,
                role TEXT DEFAULT 'subject',
                PRIMARY KEY (unit_id, entity_id),
                FOREIGN KEY (unit_id) REFERENCES memory_units(id) ON DELETE CASCADE,
                FOREIGN KEY (entity_id) REFERENCES entities(id) ON DELETE CASCADE
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS memory_links (
                id TEXT PRIMARY KEY,
                from_unit_id TEXT NOT NULL,
                to_unit_id TEXT NOT NULL,
                link_type TEXT NOT NULL,
                weight REAL NOT NULL DEFAULT 1.0,
                created_at INTEGER NOT NULL,
                FOREIGN KEY (from_unit_id) REFERENCES memory_units(id) ON DELETE CASCADE,
                FOREIGN KEY (to_unit_id) REFERENCES memory_units(id) ON DELETE CASCADE
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS entity_cooccurrences (
                entity_id_1 TEXT NOT NULL,
                entity_id_2 TEXT NOT NULL,
                cooccurrence_count INTEGER NOT NULL DEFAULT 1,
                last_cooccurred INTEGER NOT NULL,
                PRIMARY KEY (entity_id_1, entity_id_2),
                FOREIGN KEY (entity_id_1) REFERENCES entities(id) ON DELETE CASCADE,
                FOREIGN KEY (entity_id_2) REFERENCES entities(id) ON DELETE CASCADE
            )",
        ))
        .await?;

        // FTS5 全文搜索索引（用于 BM25 检索）
        db.execute(Statement::from_string(
            backend,
            "CREATE VIRTUAL TABLE IF NOT EXISTS memory_units_fts USING fts5(
                text,
                content='memory_units',
                content_rowid='id'
            )",
        ))
        .await?;

        // 索引优化
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_agent ON memory_units(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_conv ON memory_units(conversation_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_fact_type ON memory_units(fact_type)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_occurred ON memory_units(occurred_at)",
        ))
        .await?;
        db.execute(Statement::from_string(backend, "CREATE INDEX IF NOT EXISTS idx_memory_units_importance ON memory_units(importance_score DESC)")).await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_layer ON memory_units(memory_layer)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_units_expires ON memory_units(expires_at)",
        ))
        .await?;
        db.execute(Statement::from_string(backend, "CREATE INDEX IF NOT EXISTS idx_entities_agent_name ON entities(agent_id, canonical_name)")).await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_links_from ON memory_links(from_unit_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_links_to ON memory_links(to_unit_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_memory_links_type ON memory_links(link_type)",
        ))
        .await?;
        log::info!("[DB] 增强记忆系统表初始化完成");

        // ===== 定时任务表 =====
        log::info!("[DB] 初始化定时任务/钩子/凭证/规避规则表...");
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS cron_jobs (
                id TEXT PRIMARY KEY,
                agent_id TEXT,
                name TEXT NOT NULL,
                schedule TEXT NOT NULL,
                prompt TEXT NOT NULL,
                delivery_channel_id TEXT,
                delivery_chat_id TEXT,
                enabled INTEGER NOT NULL DEFAULT 1,
                silent_on_empty INTEGER NOT NULL DEFAULT 0,
                last_run_at INTEGER DEFAULT NULL,
                next_run_at INTEGER DEFAULT NULL,
                run_count INTEGER NOT NULL DEFAULT 0,
                last_result TEXT DEFAULT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_cron_jobs_agent ON cron_jobs(agent_id)",
        ))
        .await?;

        // ===== 事件钩子表 =====
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS hooks (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                event_type TEXT NOT NULL,
                handler_type TEXT NOT NULL DEFAULT 'prompt',
                handler_config TEXT NOT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                trigger_count INTEGER NOT NULL DEFAULT 0,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_hooks_agent ON hooks(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_hooks_event ON hooks(event_type)",
        ))
        .await?;

        // ===== 凭证池表 =====
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS credential_pool (
                id TEXT PRIMARY KEY,
                provider TEXT NOT NULL,
                api_key TEXT NOT NULL,
                base_url TEXT DEFAULT NULL,
                model_name TEXT DEFAULT NULL,
                weight INTEGER NOT NULL DEFAULT 1,
                is_active INTEGER NOT NULL DEFAULT 1,
                rate_limit_remaining INTEGER DEFAULT NULL,
                rate_limit_reset_at INTEGER DEFAULT NULL,
                use_count INTEGER NOT NULL DEFAULT 0,
                last_used_at INTEGER DEFAULT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_credential_pool_provider ON credential_pool(provider)",
        ))
        .await?;

        // ===== 规避规则表 =====
        db.execute(Statement::from_string(
            backend,
            "CREATE TABLE IF NOT EXISTS avoidance_rules (
                id TEXT PRIMARY KEY,
                agent_id TEXT NOT NULL,
                pattern TEXT NOT NULL,
                category TEXT NOT NULL DEFAULT 'other',
                cause TEXT NOT NULL,
                fix TEXT NOT NULL,
                trigger_count INTEGER NOT NULL DEFAULT 0,
                last_triggered_at INTEGER NOT NULL DEFAULT 0,
                is_deprecated INTEGER NOT NULL DEFAULT 0,
                expires_at INTEGER DEFAULT NULL,
                is_active INTEGER NOT NULL DEFAULT 1,
                similarity_hash TEXT DEFAULT NULL,
                created_at INTEGER NOT NULL,
                updated_at INTEGER NOT NULL
            )",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_avoidance_rules_agent ON avoidance_rules(agent_id)",
        ))
        .await?;
        db.execute(Statement::from_string(
            backend,
            "CREATE INDEX IF NOT EXISTS idx_avoidance_rules_category ON avoidance_rules(category)",
        ))
        .await?;
        log::info!("[DB] 定时任务/钩子/凭证/规避规则表初始化完成");

        db.execute(Statement::from_string(backend, "PRAGMA journal_mode=WAL"))
            .await?;
        db.execute(Statement::from_string(backend, "PRAGMA foreign_keys=ON"))
            .await?;
        log::info!("[DB] 主数据库表初始化完成");
        Ok(())
    }

    // ==================== Conversations ====================

    /// 创建新会话（自动生成 UUID + 时间戳，可选绑定 Agent ID）
    pub async fn create_conversation(
        agent_id: Option<String>,
    ) -> Result<Conversation, anyhow::Error> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        log::info!(
            "[DB] INSERT conversation: id={}",
            claw_types::truncate_str_safe(&id, 16)
        );
        let conv = crate::db::entities::conversations::ActiveModel {
            id: Set(id.clone()),
            title: Set("New Conversation".into()),
            created_at: Set(now),
            updated_at: Set(now),
            ..Default::default()
        };
        let _model = conv.insert(db).await?;

        // 如果提供了 agent_id，同时创建 agent_session 关联
        if let Some(ref aid) = agent_id {
            match try_get_agent_db() {
                Some(agent_db) => {
                    let session_id = uuid::Uuid::new_v4().to_string();
                    log::info!(
                        "[DB] INSERT agent_session for agent={} conv={}",
                        claw_types::truncate_str_safe(&aid, 8),
                        claw_types::truncate_str_safe(&id, 8)
                    );
                    let _ = crate::db::agent_entities::agent_sessions::ActiveModel {
                        id: Set(session_id),
                        agent_id: Set(aid.clone()),
                        conversation_id: Set(Some(id.clone())),
                        status: Set("active".into()),
                        turn_count: Set(0),
                        total_tokens_used: Set(0.0),
                        started_at: Set(now),
                        last_active: Set(Some(now)),
                        ..Default::default()
                    }
                    .insert(agent_db)
                    .await;
                }
                None => {}
            }
        }

        Ok(Conversation {
            id,
            title: "New Conversation".into(),
            created_at: now,
            updated_at: now,
            message_count: 0,
            agent_id,
        })
    }

    /// 列出所有会话（按 updated_at 降序，含 agent_id 联表查询）
    pub async fn list_conversations() -> Result<Vec<Conversation>, anyhow::Error> {
        let db = get_db().await;
        log::info!("[DB] SELECT * FROM conversations ORDER BY updated_at DESC");
        let models: Vec<crate::db::entities::conversations::Model> = Conversations::find()
            .order_by_desc(crate::db::entities::conversations::Column::UpdatedAt)
            .all(db)
            .await?;

        // 查询所有 agent_session 关联，构建 conversation_id → agent_id 的映射
        let mut agent_map: std::collections::HashMap<String, String> =
            std::collections::HashMap::new();
        if let Some(agent_db) = try_get_agent_db() {
            if let Ok(sessions) = crate::db::agent_entities::agent_sessions::Entity::find()
                .filter(
                    crate::db::agent_entities::agent_sessions::Column::ConversationId.is_not_null(),
                )
                .all(agent_db)
                .await
            {
                for s in sessions {
                    if let Some(ref cid) = s.conversation_id {
                        agent_map.insert(cid.clone(), s.agent_id.clone());
                    }
                }
            }
        }

        let result: Vec<Conversation> = models
            .into_iter()
            .map(|m| {
                let aid = agent_map.get(&m.id).cloned();
                Conversation {
                    id: m.id,
                    title: m.title,
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                    message_count: std::cmp::max(m.message_count, 0) as u64,
                    agent_id: aid,
                }
            })
            .collect();
        Ok(result)
    }

    /// 按 ID 获取单个会话（含 agent_id 联表查询）
    pub async fn get_conversation(id: &str) -> Result<Option<Conversation>, anyhow::Error> {
        let db = get_db().await;
        log::info!(
            "[DB] SELECT conversation WHERE id={}",
            claw_types::truncate_str_safe(id, 16)
        );
        match Conversations::find_by_id(id.to_string()).one(db).await? {
            Some(m) => {
                let aid = if let Some(agent_db) = try_get_agent_db() {
                    crate::db::agent_entities::agent_sessions::Entity::find()
                        .filter(
                            crate::db::agent_entities::agent_sessions::Column::ConversationId
                                .eq(id),
                        )
                        .one(agent_db)
                        .await
                        .ok()
                        .flatten()
                        .map(|s| s.agent_id)
                } else {
                    None
                };
                Ok(Some(Conversation {
                    id: m.id,
                    title: m.title,
                    created_at: m.created_at,
                    updated_at: m.updated_at,
                    message_count: std::cmp::max(m.message_count, 0) as u64,
                    agent_id: aid,
                }))
            }
            None => Ok(None),
        }
    }

    /// 快速获取会话关联的 Agent ID（用于模型配置覆盖）
    pub async fn get_conversation_agent_id(
        conversation_id: &str,
    ) -> Result<Option<String>, anyhow::Error> {
        match Self::get_conversation(conversation_id).await? {
            Some(conv) => Ok(conv.agent_id),
            None => Ok(None),
        }
    }

    /// 重命名会话标题
    pub async fn rename_conversation(id: &str, new_title: &str) -> Result<(), anyhow::Error> {
        let db = get_db().await;
        log::info!(
            "[DB] UPDATE conversation SET title={} WHERE id={}",
            new_title,
            claw_types::truncate_str_safe(id, 16)
        );
        if let Some(c) = Conversations::find_by_id(id.to_string()).one(db).await? {
            let mut am: crate::db::entities::conversations::ActiveModel = c.into();
            am.title = Set(new_title.to_string());
            am.updated_at = Set(chrono::Utc::now().timestamp());
            am.update(db).await?;
        }
        Ok(())
    }

    /// 删除会话及其所有消息和关联的Agent会话记录（级联删除）
    pub async fn delete_conversation(id: &str) -> Result<(), anyhow::Error> {
        let db = get_db().await;
        log::info!(
            "[DB] CASCADE DELETE conversation WHERE id={}",
            claw_types::truncate_str_safe(id, 16)
        );
        Messages::delete_many()
            .filter(crate::db::entities::messages::Column::ConversationId.eq(id))
            .exec(db)
            .await?;
        crate::db::entities::memory_units::Entity::delete_many()
            .filter(crate::db::entities::memory_units::Column::ConversationId.eq(id))
            .exec(db)
            .await?;
        Conversations::delete_by_id(id.to_string()).exec(db).await?;

        if let Some(agent_db) = crate::db::try_get_agent_db() {
            if let Ok(_) = crate::db::agent_entities::agent_sessions::Entity::delete_many()
                .filter(crate::db::agent_entities::agent_sessions::Column::ConversationId.eq(id))
                .exec(agent_db)
                .await
            {
                log::info!(
                    "[DB] Cleaned up agent_sessions for deleted conv {}",
                    claw_types::truncate_str_safe(&id, 16)
                );
            }
        }

        Ok(())
    }

    // ==================== Messages ====================

    /// 添加消息到会话（支持 token_count + metadata + 自动更新 message_count）
    pub async fn add_message(
        conv_id: &str,
        role: &str,
        content: &str,
        model: Option<&str>,
        token_count: Option<i32>,
        metadata: Option<String>,
    ) -> Result<Message, anyhow::Error> {
        let db = get_db().await;
        let now = chrono::Utc::now().timestamp();
        let id = uuid::Uuid::new_v4().to_string();
        let is_error = role == "error";
        log::info!(
            "[DB] INSERT message: conv={}, role={}, len={}, tokens={}",
            claw_types::truncate_str_safe(conv_id, 16),
            role,
            content.len(),
            token_count.unwrap_or(0)
        );
        let msg = crate::db::entities::messages::ActiveModel {
            id: Set(id.clone()),
            conversation_id: Set(conv_id.to_string()),
            role: Set(role.to_string()),
            content: Set(content.to_string()),
            timestamp: Set(now),
            is_error: Set(if is_error { 1 } else { 0 }),
            model: Set(model.map(String::from)),
            token_count: Set(token_count),
            metadata: Set(metadata),
            ..Default::default()
        };
        let model = msg.insert(db).await?;

        let count: i64 = Messages::find()
            .filter(crate::db::entities::messages::Column::ConversationId.eq(conv_id))
            .count(db)
            .await? as i64;
        if let Some(conv) = Conversations::find_by_id(conv_id.to_string())
            .one(db)
            .await?
        {
            let mut ca: crate::db::entities::conversations::ActiveModel = conv.into();
            ca.message_count = Set(count);
            ca.updated_at = Set(now);
            ca.update(db).await.ok();
        }
        Ok(Message::from(model))
    }

    /// 获取会话的所有消息（按 timestamp 升序）
    pub async fn get_messages(conversation_id: &str) -> Result<Vec<Message>, anyhow::Error> {
        let db = get_db().await;
        log::info!(
            "[DB] SELECT messages WHERE conv={}",
            claw_types::truncate_str_safe(conversation_id, 16)
        );
        let models: Vec<crate::db::entities::messages::Model> = Messages::find()
            .filter(crate::db::entities::messages::Column::ConversationId.eq(conversation_id))
            .order_by_asc(crate::db::entities::messages::Column::Timestamp)
            .all(db)
            .await?;
        Ok(models.into_iter().map(Message::from).collect())
    }

    // ==================== Import / Export（预留功能，供未来数据迁移使用） ====================

    /// 导出全部数据为 JSON 文件（conversations + messages + vectors）
    #[allow(dead_code)]
    pub async fn export_all_data() -> Result<String, anyhow::Error> {
        let db = get_db().await;
        let convs: Vec<Conversation> = Conversations::find()
            .order_by_desc(crate::db::entities::conversations::Column::UpdatedAt)
            .all(db)
            .await?
            .iter()
            .map(|c| Conversation::from(c.clone()))
            .collect();
        let mut all_msgs = Vec::new();
        for c in &convs {
            if let Ok(msgs) = Self::get_messages(&c.id).await {
                all_msgs.push((c.id.clone(), msgs));
            }
        }
        let export = serde_json::json!({ "exported_at": chrono::Utc::now().to_rfc3339(), "version": "2.0", "conversations": convs, "messages": all_msgs });
        let json_str = serde_json::to_string_pretty(&export)?;
        let file_path = format!(
            "claw_export_{}.json",
            chrono::Local::now().format("%Y%m%d_%H%M%S")
        );
        std::fs::write(&file_path, &json_str)?;
        log::info!("[DB] 导出完成: {} ({} bytes)", file_path, json_str.len());
        Ok(file_path)
    }

    /// 从 JSON 文件导入数据（覆盖模式：先清空再导入）
    #[allow(dead_code)]
    pub async fn import_data(path: &str) -> Result<(), anyhow::Error> {
        let db = get_db().await;
        let content = std::fs::read_to_string(path)?;
        let data: serde_json::Value = serde_json::from_str(&content)?;
        Messages::delete_many().exec(db).await?;
        crate::db::entities::memory_units::Entity::delete_many()
            .exec(db)
            .await?;
        Conversations::delete_many().exec(db).await?;
        if let Some(convs) = data.get("conversations").and_then(|v| v.as_array()) {
            for c in convs {
                let id = c
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let title = c
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Imported")
                    .to_string();
                use sea_orm::ActiveValue::*;
                let m = crate::db::entities::conversations::ActiveModel {
                    id: Set(id),
                    title: Set(title),
                    created_at: NotSet,
                    updated_at: NotSet,
                    message_count: Set(0),
                    metadata: NotSet,
                };
                Conversations::insert(m).exec(db).await?;
            }
        }
        log::info!("[DB] 导入完成: {}", path);
        Ok(())
    }

    // ==================== Stats ====================

    /// 获取数据库统计信息（表行数 + 文件大小）
    #[allow(dead_code)]
    pub async fn get_db_stats() -> Result<serde_json::Value, anyhow::Error> {
        let db = get_db().await;
        let conv_count = Conversations::find().count(db).await? as i64;
        let msg_count = Messages::find().count(db).await? as i64;
        let mem_count = crate::db::entities::memory_units::Entity::find()
            .count(db)
            .await? as i64;
        let db_path = std::path::PathBuf::from("claw.db");
        let file_size = std::fs::metadata(&db_path)
            .ok()
            .map(|m| m.len())
            .unwrap_or(0);
        Ok(
            serde_json::json!({ "conversations": conv_count, "messages": msg_count, "memory_units": mem_count, "db_size_bytes": file_size }),
        )
    }
}

/// 清空指定会话的所有消息（保留会话本身）
pub async fn clear_conversation_messages(conversation_id: String) -> Result<(), String> {
    let db = get_db().await;
    log::info!(
        "[DB] 清空会话消息: conv={}",
        claw_types::truncate_str_safe(&conversation_id, 16)
    );
    Messages::delete_many()
        .filter(crate::db::entities::messages::Column::ConversationId.eq(&conversation_id))
        .exec(db)
        .await
        .map_err(|e| e.to_string())?;
    if let Some(conv) = Conversations::find_by_id(conversation_id.clone())
        .one(db)
        .await
        .map_err(|e| e.to_string())?
    {
        let mut am: crate::db::entities::conversations::ActiveModel = conv.into();
        am.message_count = Set(0);
        am.updated_at = Set(chrono::Utc::now().timestamp());
        am.update(db).await.map_err(|e| e.to_string())?;
    }
    Ok(())
}
